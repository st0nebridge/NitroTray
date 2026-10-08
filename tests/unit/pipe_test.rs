//! @module pipe_test
//! @description Real named-pipe server/client round trips, concurrency, session policy, and shutdown.
use std::sync::Arc;
use std::time::{Duration, Instant};

use nitrotray::ipc::pipe::{request, IpcError, LineHandler, Server, ServerPolicy, PIPE_SDDL};

use crate::support::unique_pipe;

fn echo(name: &str) -> Server {
    let handler: LineHandler = Arc::new(|line: &str| format!("echo:{line}\n"));
    Server::spawn(name, handler).expect("server")
}

#[test]
fn round_trips_a_line() {
    let name = unique_pipe("echo");
    let server = echo(&name);
    assert_eq!(server.name(), name);
    assert_eq!(request(&name, "hello", ServerPolicy::AnySession).unwrap(), "echo:hello");
    server.stop();
}

#[test]
fn handles_bursts_of_concurrent_clients() {
    let name = unique_pipe("burst");
    let _server = echo(&name);
    let clients: Vec<_> = (0..24)
        .map(|i| {
            let n = name.clone();
            std::thread::spawn(move || request(&n, &format!("m{i}\n"), ServerPolicy::AnySession))
        })
        .collect();
    for (i, c) in clients.into_iter().enumerate() {
        assert_eq!(c.join().unwrap(), Ok(format!("echo:m{i}")));
    }
}

#[test]
fn stop_returns_promptly_even_right_after_traffic() {
    for _ in 0..20 {
        let name = unique_pipe("stop");
        let server = echo(&name);
        let _ = request(&name, "x", ServerPolicy::AnySession);
        let t = Instant::now();
        server.stop();
        assert!(t.elapsed() < Duration::from_secs(2));
        assert_eq!(request(&name, "x", ServerPolicy::AnySession), Err(IpcError::NotRunning));
    }
}

#[test]
fn service_only_policy_refuses_a_user_session_server() {
    let name = unique_pipe("policy");
    let _server = echo(&name);
    assert_eq!(request(&name, "x", ServerPolicy::ServiceOnly), Err(IpcError::UntrustedServer));
}

#[test]
fn missing_server_is_not_running() {
    assert_eq!(request(&unique_pipe("absent"), "x", ServerPolicy::AnySession), Err(IpcError::NotRunning));
}

#[test]
fn oversize_requests_are_refused_client_side() {
    let big = "x".repeat(nitrotray::ipc::protocol::MAX_MESSAGE_BYTES + 1);
    assert_eq!(request(&unique_pipe("big"), &big, ServerPolicy::AnySession), Err(IpcError::TooLarge));
}

#[test]
fn second_server_on_the_same_name_is_refused() {
    let name = unique_pipe("squat");
    let _server = echo(&name);
    let handler: LineHandler = Arc::new(|_: &str| String::new());
    assert!(matches!(Server::spawn(&name, handler), Err(IpcError::Io(_))), "FILE_FLAG_FIRST_PIPE_INSTANCE");
}

#[test]
fn acl_denies_create_instance_to_interactive_users() {
    assert!(PIPE_SDDL.contains("(A;;0x0012019b;;;IU)"));
    assert!(PIPE_SDDL.starts_with("D:P"), "protected DACL");
    for e in
        [IpcError::NotRunning, IpcError::Busy, IpcError::UntrustedServer, IpcError::TooLarge, IpcError::Io("z".into())]
    {
        assert!(!e.to_string().is_empty());
    }
}
