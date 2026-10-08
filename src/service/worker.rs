//! @module service::worker
//! @description Single hardware thread loop: serialises pipe requests and curve ticks onto one core.
//!
//! @input  `CoreMsg`s from pipe handler threads; a tick interval.
//! @output Response lines sent back on each request's reply channel; ticks between requests.
//! @dependencies service::core, acer::backend
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use crate::acer::backend::HardwareBackend;
use crate::service::core::ServiceCore;

pub enum CoreMsg {
    Line(String, Sender<String>),
    Stop,
}

/// Runs until `Stop` arrives or every sender is gone; always calls `shutdown()` on exit.
pub fn run<B: HardwareBackend>(core: &mut ServiceCore<B>, rx: &Receiver<CoreMsg>, tick: Duration) {
    let mut next_tick = Instant::now() + tick;
    loop {
        let wait = next_tick.saturating_duration_since(Instant::now());
        match rx.recv_timeout(wait) {
            Ok(CoreMsg::Line(line, reply)) => {
                let _ = reply.send(core.handle_line(&line));
            }
            Ok(CoreMsg::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
        if Instant::now() >= next_tick {
            core.tick();
            next_tick = Instant::now() + tick;
        }
    }
    core.shutdown();
}

/// Sends a line to the core thread and waits (bounded) for its answer.
pub fn submit(tx: &Sender<CoreMsg>, line: &str, timeout: Duration) -> String {
    let (reply_tx, reply_rx) = std::sync::mpsc::channel();
    if tx.send(CoreMsg::Line(line.to_string(), reply_tx)).is_err() {
        return busy_line("helper is shutting down");
    }
    reply_rx.recv_timeout(timeout).unwrap_or_else(|_| busy_line("hardware thread did not answer in time"))
}

fn busy_line(msg: &str) -> String {
    crate::ipc::protocol::encode_line(&crate::ipc::protocol::Response::fail(
        crate::ipc::protocol::ErrorCode::Internal,
        msg,
    ))
}
