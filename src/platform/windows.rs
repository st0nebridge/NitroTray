//! @module platform::windows
//! @description Thin Win32 helpers: registry values, autostart Run entry, shell launch, OS/model info, single instance.
//!
//! @input  Registry paths/values, shell targets.
//! @output Strings/DWORDs read from the registry; side effects only where the function name says so.
//! @dependencies windows (Registry, Shell, Threading, Foundation)
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
use windows::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, REG_DWORD, REG_SZ,
    RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// Registry hive selector (keeps `HKEY` out of callers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hive {
    LocalMachine,
    CurrentUser,
}

fn hkey(h: Hive) -> HKEY {
    match h {
        Hive::LocalMachine => HKEY_LOCAL_MACHINE,
        Hive::CurrentUser => HKEY_CURRENT_USER,
    }
}

pub fn read_string(hive: Hive, subkey: &str, value: &str) -> Option<String> {
    let (k, v) = (HSTRING::from(subkey), HSTRING::from(value));
    let mut buf = vec![0u16; 512];
    let mut size = (buf.len() * 2) as u32;
    // SAFETY: buffer and size describe the same allocation.
    let rc = unsafe {
        RegGetValueW(hkey(hive), &k, &v, RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr().cast()), Some(&mut size))
    };
    if rc.is_err() {
        return None;
    }
    let len = (size as usize / 2).saturating_sub(1).min(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]).trim_end_matches('\0').to_string())
}

pub fn read_dword(hive: Hive, subkey: &str, value: &str) -> Option<u32> {
    let (k, v) = (HSTRING::from(subkey), HSTRING::from(value));
    let mut data = 0u32;
    let mut size = 4u32;
    // SAFETY: a DWORD-sized destination.
    let rc = unsafe {
        RegGetValueW(hkey(hive), &k, &v, RRF_RT_REG_DWORD, None, Some((&mut data as *mut u32).cast()), Some(&mut size))
    };
    rc.is_ok().then_some(data)
}

pub fn write_string(hive: Hive, subkey: &str, value: &str, data: &str) -> Result<(), String> {
    let (k, v) = (HSTRING::from(subkey), HSTRING::from(value));
    let wide: Vec<u16> = data.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: pointer/length describe `wide`.
    let rc =
        unsafe { RegSetKeyValueW(hkey(hive), &k, &v, REG_SZ.0, Some(wide.as_ptr().cast()), (wide.len() * 2) as u32) };
    rc.ok().map_err(|e| format!("registry write failed: {e}"))
}

pub fn write_dword(hive: Hive, subkey: &str, value: &str, data: u32) -> Result<(), String> {
    let (k, v) = (HSTRING::from(subkey), HSTRING::from(value));
    // SAFETY: pointer/length describe `data`.
    let rc = unsafe { RegSetKeyValueW(hkey(hive), &k, &v, REG_DWORD.0, Some((&data as *const u32).cast()), 4) };
    rc.ok().map_err(|e| format!("registry write failed: {e}"))
}

pub fn delete_value(hive: Hive, subkey: &str, value: &str) -> Result<(), String> {
    let (k, v) = (HSTRING::from(subkey), HSTRING::from(value));
    // SAFETY: plain registry call.
    let rc = unsafe { RegDeleteKeyValueW(hkey(hive), &k, &v) };
    match rc.0 {
        0 | 2 => Ok(()), // deleted, or already absent
        _ => Err(format!("registry delete failed: {}", rc.0)),
    }
}

/// The HKCU Run entry that starts NitroTray with Windows.
#[derive(Debug, Clone)]
pub struct Autostart {
    pub subkey: String,
    pub value: String,
}

pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

impl Default for Autostart {
    fn default() -> Self {
        Self { subkey: RUN_KEY.into(), value: crate::meta::APP_NAME.into() }
    }
}

/// Command line registered for autostart.
pub fn autostart_command(exe: &str) -> String {
    format!("\"{exe}\" --autostart")
}

impl Autostart {
    pub fn command(&self) -> Option<String> {
        read_string(Hive::CurrentUser, &self.subkey, &self.value)
    }

    pub fn is_enabled(&self) -> bool {
        self.command().is_some()
    }

    pub fn set(&self, enabled: bool, exe: &str) -> Result<(), String> {
        if enabled {
            write_string(Hive::CurrentUser, &self.subkey, &self.value, &autostart_command(exe))
        } else {
            delete_value(Hive::CurrentUser, &self.subkey, &self.value)
        }
    }
}

fn shell_execute(verb: &str, file: &str, params: Option<&str>) -> Result<(), String> {
    let (verb, file) = (HSTRING::from(verb), HSTRING::from(file));
    let params = params.map(HSTRING::from);
    let params_ptr = params.as_ref().map_or(PCWSTR::null(), |p| PCWSTR(p.as_ptr()));
    // SAFETY: all strings outlive the call.
    let h = unsafe { ShellExecuteW(None, &verb, &file, params_ptr, PCWSTR::null(), SW_SHOWNORMAL) };
    if h.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("shell refused to open '{file}' (code {})", h.0 as isize))
    }
}

/// Opens a file, folder, URL or `shell:` target with its default handler.
pub fn shell_open(target: &str) -> Result<(), String> {
    shell_execute("open", target, None)
}

/// Starts a program elevated; Windows shows the UAC consent prompt to the user.
pub fn run_elevated(exe: &str, args: &str) -> Result<(), String> {
    shell_execute("runas", exe, Some(args))
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct MachineInfo {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub bios_version: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub os_build: Option<String>,
}

pub fn machine_info() -> MachineInfo {
    const BIOS: &str = r"HARDWARE\DESCRIPTION\System\BIOS";
    const NT: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    let build =
        read_string(Hive::LocalMachine, NT, "CurrentBuild").map(|b| match read_dword(Hive::LocalMachine, NT, "UBR") {
            Some(ubr) => format!("{b}.{ubr}"),
            None => b,
        });
    MachineInfo {
        manufacturer: read_string(Hive::LocalMachine, BIOS, "SystemManufacturer"),
        model: read_string(Hive::LocalMachine, BIOS, "SystemProductName"),
        bios_version: read_string(Hive::LocalMachine, BIOS, "BIOSVersion"),
        os_name: read_string(Hive::LocalMachine, NT, "ProductName"),
        os_version: read_string(Hive::LocalMachine, NT, "DisplayVersion"),
        os_build: build,
    }
}

/// True when Windows apps are set to the light theme.
pub fn apps_use_light_theme() -> bool {
    read_dword(Hive::CurrentUser, r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "AppsUseLightTheme")
        == Some(1)
}

/// Named-mutex guard; the second holder of the same name is refused.
pub struct SingleInstance(HANDLE);

impl SingleInstance {
    pub fn acquire(name: &str) -> Option<Self> {
        let name = HSTRING::from(name);
        // SAFETY: plain mutex creation; the handle is closed on drop.
        unsafe {
            let handle = CreateMutexW(None, true, &name).ok()?;
            if GetLastError() == ERROR_ALREADY_EXISTS {
                let _ = CloseHandle(handle);
                return None;
            }
            Some(Self(handle))
        }
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        // SAFETY: the handle came from CreateMutexW and is closed exactly once.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
