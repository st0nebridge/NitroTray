//! @module binaries_test
//! @description The shipped executables: helper CLI errors, console-mode lifecycle over a real pipe
//! (simulated backend), and `NitroTray.exe --exit` with nothing running.
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use nitrotray::ipc::client::{HelperClient, PipeTransport};
use nitrotray::ipc::pipe::ServerPolicy;
use nitrotray::ipc::protocol::ProviderStatus;

use crate::support::unique_pipe;

const HELPER: &str = env!("CARGO_BIN_EXE_NitroTrayService");
const TRAY: &str = env!("CARGO_BIN_EXE_NitroTray");

#[test]
fn helper_cli_help_and_errors() {
    let help = Command::new(HELPER).arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("usage"));
    let bad = Command::new(HELPER).arg("explode").output().unwrap();
    assert_eq!(bad.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&bad.stderr).contains("unknown command"));
}

#[test]
fn helper_console_serves_until_q() {
    let pipe = unique_pipe("bin");
    let mut child = Command::new(HELPER)
        .args(["console", "--simulate", "--pipe", &pipe])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let client = HelperClient::new(PipeTransport { name: pipe.clone(), policy: ServerPolicy::AnySession });
    let deadline = Instant::now() + Duration::from_secs(10);
    let caps = loop {
        match client.capabilities() {
            Ok(c) => break c,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => panic!("helper never answered: {e:?}"),
        }
    };
    assert_eq!(caps.status, ProviderStatus::Simulated);
    child.stdin.as_mut().unwrap().write_all(b"q\n").unwrap();
    let status = child.wait().unwrap();
    assert!(status.success());
    assert!(client.capabilities().is_err(), "pipe closed after q");
}

#[test]
fn helper_refuses_a_taken_pipe() {
    let pipe = unique_pipe("taken");
    let _server = nitrotray::ipc::pipe::Server::spawn(&pipe, std::sync::Arc::new(|_: &str| String::new())).unwrap();
    let out =
        Command::new(HELPER).args(["console", "--simulate", "--pipe", &pipe]).stdin(Stdio::null()).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot serve"));
}

/// Runs NitroTray.exe with a private instance tag so a test can never signal a real running tray.
fn tray(tag: &str, arg: &str) -> std::io::Result<std::process::ExitStatus> {
    Command::new(TRAY).arg(arg).env("NITROTRAY_INSTANCE", tag).status()
}

#[test]
fn tray_exit_flag_signals_only_its_own_instance_scope() {
    use nitrotray::meta::scoped;
    use nitrotray::platform::signal::{ShowSignal, EXIT_EVENT};
    let tag = format!("test{}", std::process::id());
    let ours = ShowSignal::create(&scoped(EXIT_EVENT, Some(&tag))).unwrap();
    let other = ShowSignal::create(&scoped(EXIT_EVENT, Some(&format!("{tag}x")))).unwrap();
    match tray(&tag, "--exit") {
        Ok(s) => {
            assert!(s.success());
            assert!(ours.wait(2000), "tagged instance was signalled");
            assert!(!other.wait(0), "no other scope was touched");
        }
        // Defender ASR may refuse new unsigned builds on this machine; that is environment, not product.
        Err(e) => assert_eq!(e.raw_os_error(), Some(5), "{e}"),
    }
}
