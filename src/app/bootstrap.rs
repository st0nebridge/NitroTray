//! @module app::bootstrap
//! @description Process start-up: single-instance guard, `--exit`, settings load, worker threads,
//! OS callback forwarding into the event loop, then hands control to the runtime.
//!
//! @input  `Options` from the command line.
//! @output A running event loop (never returns on success); Ok(()) for second instances / `--exit`.
//! @dependencies tao, tray-icon, global-hotkey, app::{events, runtime, worker}, config, ipc, platform, telemetry
use std::sync::mpsc::{self, Sender};
use std::time::Instant;

use global_hotkey::GlobalHotKeyEvent;
use tao::event_loop::{EventLoopBuilder, EventLoopProxy};
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIconEvent;
use windows::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY};

use crate::app::events::{Options, UserEvent};
use crate::app::model::ControlRequest;
use crate::app::runtime::Runtime;
use crate::app::worker::{self, TelemetryConfig, WorkerMsg};
use crate::config::{store, ConfigStore, Settings};
use crate::ipc::client::{HelperClient, PipeTransport};
use crate::meta;
use crate::platform::signal::{notify_existing, ShowSignal, EXIT_EVENT, SHOW_EVENT};
use crate::platform::windows::SingleInstance;
use crate::telemetry::{cpu::PdhCpu, gpu::LazyNvml, CpuSource, GpuSource, TelemetryService};

fn spawn_workers(
    proxy: &EventLoopProxy<UserEvent>,
    settings: &Settings,
    started: Instant,
) -> (Sender<WorkerMsg>, Sender<ControlRequest>) {
    let (wtx, wrx) = mpsc::channel();
    let cfg = TelemetryConfig::from_settings(settings);
    let p = proxy.clone();
    std::thread::spawn(move || {
        let cpu: Option<Box<dyn CpuSource>> = PdhCpu::open().ok().map(|c| Box::new(c) as Box<dyn CpuSource>);
        let gpu: Option<Box<dyn GpuSource>> = Some(Box::new(LazyNvml::default()));
        let svc = TelemetryService::new(cpu, gpu, Box::new(HelperClient::new(PipeTransport::from_env())));
        worker::run_telemetry(
            svc,
            &wrx,
            cfg,
            move || started.elapsed().as_millis() as u64,
            move |d| {
                let _ = p.send_event(UserEvent::Telemetry(Box::new(d)));
            },
        );
    });
    let (ctx, crx) = mpsc::channel::<ControlRequest>();
    let p = proxy.clone();
    std::thread::spawn(move || {
        let client = HelperClient::new(PipeTransport::from_env());
        worker::run_control(&client, &crx, move |req, result| {
            let _ = p.send_event(UserEvent::ControlDone(req, result));
        });
    });
    (wtx, ctx)
}

fn forward_callbacks(proxy: &EventLoopProxy<UserEvent>, show_event: String, exit_event: String) {
    let p = proxy.clone();
    TrayIconEvent::set_event_handler(Some(move |e| {
        let _ = p.send_event(UserEvent::Tray(e));
    }));
    let p = proxy.clone();
    MenuEvent::set_event_handler(Some(move |e| {
        let _ = p.send_event(UserEvent::Menu(e));
    }));
    let p = proxy.clone();
    GlobalHotKeyEvent::set_event_handler(Some(move |e| {
        let _ = p.send_event(UserEvent::Hotkey(e));
    }));
    for (name, make) in
        [(show_event, (|| UserEvent::ShowRequested) as fn() -> UserEvent), (exit_event, || UserEvent::ExitRequested)]
    {
        let p = proxy.clone();
        std::thread::spawn(move || {
            let Some(signal) = ShowSignal::create(&name) else { return };
            loop {
                if signal.wait(u32::MAX) && p.send_event(make()).is_err() {
                    break;
                }
            }
        });
    }
}

/// Runs the tray host until Exit. A second instance just asks the first to show its popup.
pub fn run(opts: Options) -> Result<(), String> {
    let tag = meta::instance_tag();
    let (show_event, exit_event) = (meta::scoped(SHOW_EVENT, tag.as_deref()), meta::scoped(EXIT_EVENT, tag.as_deref()));
    if opts.exit_running {
        notify_existing(&exit_event);
        return Ok(());
    }
    let Some(instance) = SingleInstance::acquire(&meta::scoped(meta::INSTANCE_MUTEX, tag.as_deref())) else {
        // This process was just launched by the user, so it may hand foreground rights to the
        // running instance; without that its popup could not take focus (and dismiss on click-away).
        // SAFETY: plain call with the documented ASFW_ANY sentinel.
        unsafe {
            let _ = AllowSetForegroundWindow(ASFW_ANY);
        }
        notify_existing(&show_event);
        return Ok(());
    };
    let store = ConfigStore::new(store::default_dir());
    let (settings, warning) = store.load();
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    forward_callbacks(&proxy, show_event, exit_event);
    let started = Instant::now();
    let (worker_tx, control_tx) = spawn_workers(&proxy, &settings, started);
    let mut rt = Runtime::new(settings, store, proxy, worker_tx, control_tx, started, opts, warning);
    event_loop.run(move |event, target, control_flow| {
        let _keep = &instance;
        rt.handle(event, target, control_flow);
    })
}
