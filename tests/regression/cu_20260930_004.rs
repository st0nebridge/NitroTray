//! @module cu_20260930_004
//! @description Regression test: the pipe server never hangs on stop (it previously could stay blocked in
//! ConnectNamedPipe and stall process exit), and bursts of clients are not refused as busy.
use std::sync::Arc;
use std::time::{Duration, Instant};

use nitrotray::ipc::pipe::{request, LineHandler, Server, ServerPolicy};

fn name(tag: &str, i: usize) -> String {
    format!(r"\\.\pipe\NitroTrayRegression-{tag}-{}-{i}", std::process::id())
}

#[test]
fn stop_after_burst_is_prompt() {
    for round in 0..10 {
        let n = name("burst", round);
        let handler: LineHandler = Arc::new(|l: &str| format!("{l}\n"));
        let server = Server::spawn(&n, handler).unwrap();
        let clients: Vec<_> = (0..16)
            .map(|i| {
                let n = n.clone();
                std::thread::spawn(move || request(&n, &i.to_string(), ServerPolicy::AnySession))
            })
            .collect();
        for (i, c) in clients.into_iter().enumerate() {
            assert_eq!(c.join().unwrap(), Ok(i.to_string()));
        }
        let t = Instant::now();
        server.stop();
        assert!(t.elapsed() < Duration::from_secs(2), "round {round}");
    }
}

#[test]
fn dropping_a_server_releases_the_pipe() {
    let n = name("drop", 0);
    let handler: LineHandler = Arc::new(|l: &str| format!("{l}\n"));
    drop(Server::spawn(&n, handler.clone()).unwrap());
    assert!(Server::spawn(&n, handler).is_ok(), "name is free again after drop");
}
