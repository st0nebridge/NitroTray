//! @module telemetry
//! @description Telemetry subsystem: providers, snapshots, histories, scheduling and colour thresholds.
//!
//! @dependencies telemetry::{collector, cpu, fan, gpu, history, scheduler, snapshot, thresholds}
pub mod collector;
pub mod cpu;
pub mod fan;
pub mod gpu;
pub mod history;
pub mod scheduler;
pub mod snapshot;
pub mod thresholds;

pub use collector::{
    CpuSample, CpuSource, GpuSample, GpuSource, HardwareSource, PollOptions, ProviderErrorEntry, TelemetryService,
};
