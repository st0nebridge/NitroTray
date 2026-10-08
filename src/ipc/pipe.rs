//! @module ipc::pipe
//! @description Local, ACL-restricted named-pipe transport: a threaded line server and a one-shot client.
//!
//! @input  Pipe name; a line handler (server) or a request line (client).
//! @output A `Server` handle (stops on drop) and response lines; `IpcError` on transport failure or an
//!         untrusted server.
//! @dependencies windows (Pipes, FileSystem, Security), meta
//!
//! Security: remote clients rejected, DACL grants interactive users read/write
//! but not create-instance (0x0012019B), clients connect at SECURITY_IDENTIFICATION so a
//! squatting server cannot impersonate them, and by default the client only trusts a server
//! running in session 0 (i.e. a service).
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use windows::core::HSTRING;
use windows::Win32::Foundation::{CloseHandle, LocalFree, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, HANDLE, HLOCAL};
use windows::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FlushFileBuffers, ReadFile, WriteFile, FILE_FLAGS_AND_ATTRIBUTES, FILE_FLAG_FIRST_PIPE_INSTANCE,
    FILE_SHARE_NONE, OPEN_EXISTING, PIPE_ACCESS_DUPLEX, SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeServerSessionId, WaitNamedPipeW,
    PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};

use super::protocol::MAX_MESSAGE_BYTES;

/// SYSTEM/Administrators/owner full control; interactive users read+write without create-instance.
pub const PIPE_SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)(A;;0x0012019b;;;IU)";
const FILE_GENERIC_READ: u32 = 0x0012_0089;
const FILE_WRITE_DATA: u32 = 0x0000_0002;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcError {
    /// No server is listening (helper not installed or stopped).
    NotRunning,
    /// All instances busy for longer than the wait budget.
    Busy,
    /// The server is not in session 0 while a service was required.
    UntrustedServer,
    TooLarge,
    Io(String),
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRunning => write!(f, "the NitroTray helper is not running"),
            Self::Busy => write!(f, "the NitroTray helper is busy"),
            Self::UntrustedServer => {
                write!(f, "refused a pipe server that is not the NitroTray service")
            }
            Self::TooLarge => write!(f, "message exceeded the size limit"),
            Self::Io(m) => write!(f, "pipe error: {m}"),
        }
    }
}

/// Which servers the client accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerPolicy {
    /// Only a server in session 0 (a Windows service).
    ServiceOnly,
    /// Any local server (development console helper).
    AnySession,
}

struct Handle(HANDLE);

// SAFETY: a kernel handle is not bound to the thread that opened it.
unsafe impl Send for Handle {}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: closes a handle this wrapper owns.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// Owned security descriptor built from `PIPE_SDDL`.
struct Descriptor(PSECURITY_DESCRIPTOR);

impl Descriptor {
    fn new(sddl: &str) -> Result<Self, IpcError> {
        let mut sd = PSECURITY_DESCRIPTOR::default();
        // SAFETY: the API allocates `sd`, which Drop frees with LocalFree.
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(&HSTRING::from(sddl), 1, &mut sd, None)
                .map_err(|e| IpcError::Io(format!("security descriptor: {e}")))?;
        }
        Ok(Self(sd))
    }
}

impl Drop for Descriptor {
    fn drop(&mut self) {
        // SAFETY: frees the LocalAlloc'd descriptor exactly once.
        unsafe {
            let _ = LocalFree(Some(HLOCAL(self.0 .0)));
        }
    }
}

fn read_line(h: HANDLE) -> Result<String, IpcError> {
    let mut data = Vec::with_capacity(256);
    let mut chunk = [0u8; 1024];
    loop {
        let mut n = 0u32;
        // SAFETY: reads into a stack buffer of the stated length.
        let res = unsafe { ReadFile(h, Some(&mut chunk), Some(&mut n), None) };
        if res.is_err() || n == 0 {
            break;
        }
        data.extend_from_slice(&chunk[..n as usize]);
        if data.len() > MAX_MESSAGE_BYTES {
            return Err(IpcError::TooLarge);
        }
        if data.contains(&b'\n') {
            break;
        }
    }
    let end = data.iter().position(|&b| b == b'\n').unwrap_or(data.len());
    String::from_utf8(data[..end].to_vec()).map_err(|_| IpcError::Io("message is not UTF-8".into()))
}

fn write_all(h: HANDLE, bytes: &[u8]) -> Result<(), IpcError> {
    let mut written = 0u32;
    // SAFETY: writes from a borrowed slice.
    unsafe {
        WriteFile(h, Some(bytes), Some(&mut written), None).map_err(|e| IpcError::Io(e.to_string()))?;
        let _ = FlushFileBuffers(h);
    }
    Ok(())
}

pub type LineHandler = Arc<dyn Fn(&str) -> String + Send + Sync>;

// SAFETY: the descriptor is immutable after creation and only freed on drop.
unsafe impl Send for Descriptor {}

fn create_instance(name: &HSTRING, descriptor: &Descriptor, first: bool) -> Result<Handle, IpcError> {
    let sa = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0 .0,
        bInheritHandle: false.into(),
    };
    let open_mode = if first { PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE } else { PIPE_ACCESS_DUPLEX };
    // SAFETY: name and attributes outlive the call.
    let h = unsafe {
        CreateNamedPipeW(
            name,
            open_mode,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            MAX_MESSAGE_BYTES as u32,
            MAX_MESSAGE_BYTES as u32,
            0,
            Some(&sa),
        )
    };
    if h.is_invalid() {
        Err(IpcError::Io(format!("CreateNamedPipe failed for {name}")))
    } else {
        Ok(Handle(h))
    }
}

fn answer(pipe: Handle, handler: &LineHandler) {
    let reply = match read_line(pipe.0) {
        Ok(line) => handler(&line),
        Err(e) => super::protocol::encode_line(&super::protocol::Response::fail(
            super::protocol::ErrorCode::InvalidRequest,
            e.to_string(),
        )),
    };
    let _ = write_all(pipe.0, reply.as_bytes());
    // SAFETY: disconnect our own instance before the handle drops.
    unsafe {
        let _ = DisconnectNamedPipe(pipe.0);
    }
}

/// Accepts connections; the next listening instance is created before a connected one is
/// handed off, so a running server never has zero instances (clients never see "not found").
fn accept_loop(
    name: HSTRING,
    descriptor: Descriptor,
    mut listener: Handle,
    handler: LineHandler,
    stop: Arc<AtomicBool>,
) {
    loop {
        // SAFETY: blocking accept on our own instance.
        let connected = match unsafe { ConnectNamedPipe(listener.0, None) } {
            Ok(()) => true,
            Err(e) => e.code() == ERROR_PIPE_CONNECTED.to_hresult(),
        };
        if stop.load(Ordering::SeqCst) {
            return;
        }
        let conn = listener;
        listener = loop {
            match create_instance(&name, &descriptor, false) {
                Ok(h) => break h,
                Err(_) if stop.load(Ordering::SeqCst) => return,
                Err(_) => std::thread::sleep(Duration::from_millis(50)),
            }
        };
        if connected {
            let handler = Arc::clone(&handler);
            std::thread::spawn(move || answer(conn, &handler));
        }
    }
}

/// A running pipe server. Stops (and joins) on [`Server::stop`] or drop, so a server can never
/// be left blocked in an accept.
pub struct Server {
    name: String,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    /// Creates the first instance synchronously (so a squatter or bad name fails here), then
    /// serves on a background thread.
    pub fn spawn(name: &str, handler: LineHandler) -> Result<Self, IpcError> {
        let descriptor = Descriptor::new(PIPE_SDDL)?;
        let wname = HSTRING::from(name);
        let listener = create_instance(&wname, &descriptor, true)?;
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let thread = std::thread::spawn(move || accept_loop(wname, descriptor, listener, handler, flag));
        Ok(Self { name: name.to_string(), stop, thread: Some(thread) })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        let Some(thread) = self.thread.take() else { return };
        self.stop.store(true, Ordering::SeqCst);
        for _ in 0..200 {
            if thread.is_finished() {
                break;
            }
            let _ = open_client(&self.name, Duration::from_millis(20));
            std::thread::sleep(Duration::from_millis(5));
        }
        if thread.is_finished() {
            let _ = thread.join();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}

const ERROR_FILE_NOT_FOUND_HR: u32 = 0x8007_0002;

fn open_client(name: &str, budget: Duration) -> Result<Handle, IpcError> {
    let wname = HSTRING::from(name);
    let deadline = Instant::now() + budget;
    loop {
        // SAFETY: opening a pipe client handle that the caller owns via `Handle`.
        let res = unsafe {
            CreateFileW(
                &wname,
                FILE_GENERIC_READ | FILE_WRITE_DATA,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(SECURITY_SQOS_PRESENT.0 | SECURITY_IDENTIFICATION.0),
                None,
            )
        };
        let left = deadline.saturating_duration_since(Instant::now());
        match res {
            Ok(h) => return Ok(Handle(h)),
            Err(e) if e.code() == ERROR_PIPE_BUSY.to_hresult() => {
                if left.is_zero() {
                    return Err(IpcError::Busy);
                }
                let wait = left.as_millis().clamp(1, 250) as u32;
                // SAFETY: waits on a named pipe by name.
                let _ = unsafe { WaitNamedPipeW(&wname, wait) };
            }
            // A running `Server` always keeps a listening instance (the next one is created before a
            // connected one is handed off), so "not found" — even right after "busy" — means it stopped.
            Err(e) if e.code().0 as u32 == ERROR_FILE_NOT_FOUND_HR => return Err(IpcError::NotRunning),
            Err(e) => return Err(IpcError::Io(e.to_string())),
        }
    }
}

/// Sends one request line and returns the response line.
pub fn request(name: &str, line: &str, policy: ServerPolicy) -> Result<String, IpcError> {
    if line.len() > MAX_MESSAGE_BYTES {
        return Err(IpcError::TooLarge);
    }
    let pipe = open_client(name, Duration::from_secs(2))?;
    if policy == ServerPolicy::ServiceOnly {
        let mut session = u32::MAX;
        // SAFETY: queries the server's session id into a local.
        let ok = unsafe { GetNamedPipeServerSessionId(pipe.0, &mut session) }.is_ok();
        if !ok || session != 0 {
            return Err(IpcError::UntrustedServer);
        }
    }
    let mut msg = line.trim_end().to_string();
    msg.push('\n');
    write_all(pipe.0, msg.as_bytes())?;
    read_line(pipe.0)
}
