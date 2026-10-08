//! @module service::host
//! @description Windows host for the helper: SCM service entry, console mode, install/uninstall
//! (install copies the binary into the admin-only Program Files location first).
//!
//! @input  `service::cli::Command`.
//! @output A running pipe server + hardware thread; SCM registration side effects for install/uninstall.
//! @dependencies windows-service, ipc::pipe, service::{core, worker, simulated}, acer::{gaming, wmi}, platform::windows
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use windows_service::service::{
    ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode, ServiceInfo,
    ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
use windows_service::{define_windows_service, service_dispatcher};

use crate::acer::backend::{HardwareBackend, UnavailableBackend};
use crate::acer::gaming::AcerGaming;
use crate::acer::wmi::WmiGamingExecutor;
use crate::ipc::pipe;
use crate::ipc::protocol::ProviderStatus;
use crate::meta;
use crate::service::core::{status_for, ServiceCore};
use crate::service::curve::TICK_MS;
use crate::service::install_path;
use crate::service::simulated::SimulatedBackend;
use crate::service::worker::{self, CoreMsg};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Marker proving a custom curve is live; survives a crash so the next start restores Auto.
/// The production service keeps it in its administrators-only Program Files folder: SYSTEM must never
/// write where a standard user can create the folder first and plant a link (`%ProgramData%` allows
/// both). Development helpers get one per pipe name under `%ProgramData%\NitroTray`. `None` (no crash
/// recovery) only if Program Files is unknown.
pub fn marker_path(pipe_name: &str, simulate: bool) -> Option<PathBuf> {
    if pipe_name == meta::PIPE_NAME && !simulate {
        return install_path::program_files().map(|pf| install_path::service_dir(&pf).join("curve-active"));
    }
    let base = std::env::var_os("ProgramData").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let tag: String = pipe_name.chars().filter(char::is_ascii_alphanumeric).collect();
    Some(base.join(meta::APP_NAME).join(format!("curve-active-dev-{tag}")))
}

/// A running helper: pipe server + hardware thread.
pub struct Runner {
    server: Option<pipe::Server>,
    tx: Sender<CoreMsg>,
    core: Option<JoinHandle<()>>,
}

impl Runner {
    /// Starts the hardware thread and the pipe server; fails if the pipe cannot be created
    /// (e.g. another helper already owns it).
    pub fn start(pipe_name: &str, simulate: bool) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<CoreMsg>();
        let marker = marker_path(pipe_name, simulate);
        let core = std::thread::spawn(move || {
            let model = crate::platform::windows::machine_info().model.unwrap_or_default();
            let (backend, status): (Box<dyn HardwareBackend>, ProviderStatus) = if simulate {
                (Box::new(SimulatedBackend::default()), ProviderStatus::Simulated)
            } else {
                match WmiGamingExecutor::connect() {
                    Ok(exec) => (Box::new(AcerGaming::new(exec, model.clone())), ProviderStatus::Connected),
                    Err(e) => (Box::new(UnavailableBackend(e.clone())), status_for(&e)),
                }
            };
            let mut core = ServiceCore::new(backend, model, status, marker);
            worker::run(&mut core, &rx, Duration::from_millis(TICK_MS));
        });
        let submit_tx = tx.clone();
        let handler: pipe::LineHandler = Arc::new(move |line: &str| worker::submit(&submit_tx, line, REQUEST_TIMEOUT));
        match pipe::Server::spawn(pipe_name, handler) {
            Ok(server) => Ok(Self { server: Some(server), tx, core: Some(core) }),
            Err(e) => {
                let _ = tx.send(CoreMsg::Stop);
                let _ = core.join();
                Err(format!("cannot serve {pipe_name}: {e}"))
            }
        }
    }

    /// Stops accepting, restores Auto if a curve was running, and joins both threads.
    pub fn stop(mut self) {
        if let Some(server) = self.server.take() {
            server.stop();
        }
        let _ = self.tx.send(CoreMsg::Stop);
        if let Some(h) = self.core.take() {
            let _ = h.join();
        }
    }
}

/// Foreground mode: serves until stdin reaches EOF or a line reading `q`.
pub fn run_console(simulate: bool, pipe_name: Option<String>) -> Result<(), String> {
    let name = pipe_name.unwrap_or_else(|| meta::PIPE_NAME.to_string());
    println!(
        "{} helper (console{}) on {name} — type q + Enter to stop",
        meta::APP_NAME,
        if simulate { ", simulated" } else { "" }
    );
    let runner = Runner::start(&name, simulate)?;
    let mut line = String::new();
    loop {
        line.clear();
        match std::io::stdin().read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) if line.trim().eq_ignore_ascii_case("q") => break,
            Ok(_) => {}
        }
    }
    runner.stop();
    Ok(())
}

define_windows_service!(ffi_service_main, service_main);

fn service_main(_args: Vec<OsString>) {
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
    let handler = move |control| match control {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            let _ = shutdown_tx.send(());
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    };
    let Ok(status) = service_control_handler::register(meta::SERVICE_NAME, handler) else {
        return;
    };
    let report = |state, accept| ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: accept,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(3),
        process_id: None,
    };
    let _ = status
        .set_service_status(report(ServiceState::Running, ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN));
    let runner = match Runner::start(meta::PIPE_NAME, false) {
        Ok(r) => r,
        Err(_) => {
            let _ = status.set_service_status(report(ServiceState::Stopped, ServiceControlAccept::empty()));
            return;
        }
    };
    let _ = shutdown_rx.recv();
    let _ = status.set_service_status(report(ServiceState::StopPending, ServiceControlAccept::empty()));
    runner.stop();
    let _ = status.set_service_status(report(ServiceState::Stopped, ServiceControlAccept::empty()));
}

/// Entry used by the Service Control Manager.
pub fn run_service() -> Result<(), String> {
    service_dispatcher::start(meta::SERVICE_NAME, ffi_service_main).map_err(|e| format!("service dispatcher: {e}"))
}

fn service_info(exe: std::path::PathBuf) -> ServiceInfo {
    ServiceInfo {
        name: OsString::from(meta::SERVICE_NAME),
        display_name: OsString::from("NitroTray Helper"),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: exe,
        launch_arguments: vec![OsString::from("run")],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    }
}

fn wait_stopped(service: &windows_service::service::Service) {
    for _ in 0..40 {
        if service.query_status().map(|s| s.current_state == ServiceState::Stopped).unwrap_or(true) {
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// Copies this binary into `%ProgramFiles%\NitroTray` (admin-only) and registers/updates the
/// LocalSystem service to run **that** copy. Requires elevation.
pub fn install() -> Result<(), String> {
    let current = std::env::current_exe().map_err(|e| e.to_string())?;
    let pf = install_path::program_files().ok_or("ProgramFiles is not set")?;
    let target = install_path::service_exe(&pf);
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )
    .map_err(|e| format!("cannot open the service manager (run elevated): {e}"))?;
    let access =
        ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::START | ServiceAccess::CHANGE_CONFIG;
    let existing = manager.open_service(meta::SERVICE_NAME, access).ok();
    if let Some(service) = &existing {
        let _ = service.stop();
        wait_stopped(service);
    }
    std::fs::create_dir_all(install_path::service_dir(&pf)).map_err(|e| format!("create {}: {e}", pf.display()))?;
    if !same_file(&current, &target) {
        std::fs::copy(&current, &target).map_err(|e| format!("copy to {}: {e}", target.display()))?;
    }
    let info = service_info(target);
    let service = match existing {
        Some(s) => {
            s.change_config(&info).map_err(|e| format!("update service: {e}"))?;
            s
        }
        None => manager.create_service(&info, access).map_err(|e| format!("create service: {e}"))?,
    };
    let _ = service.set_description("Narrow, typed access to Acer gaming firmware for the NitroTray tray companion.");
    service.start::<&str>(&[]).map_err(|e| format!("start service: {e}"))
}

fn same_file(a: &std::path::Path, b: &std::path::Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

pub fn uninstall() -> Result<(), String> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|e| format!("cannot open the service manager: {e}"))?;
    let service = manager
        .open_service(meta::SERVICE_NAME, ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS)
        .map_err(|e| format!("open service (run elevated): {e}"))?;
    let _ = service.stop();
    wait_stopped(&service);
    service.delete().map_err(|e| format!("delete service: {e}"))?;
    if let Some(pf) = install_path::program_files() {
        let target = install_path::service_exe(&pf);
        let current = std::env::current_exe().unwrap_or_default();
        if !same_file(&current, &target) {
            let _ = std::fs::remove_file(&target);
            let _ = std::fs::remove_dir(install_path::service_dir(&pf));
        }
    }
    Ok(())
}
