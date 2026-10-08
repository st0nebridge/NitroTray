//! @module telemetry::history
//! @description Fixed-capacity sample histories for tray graph icons and the monitoring view.
//!
//! @input  Snapshots (or single optional samples) pushed in time order.
//! @output Ordered sample windows (oldest first), peaks, and a serialisable multi-series view.
//! @dependencies serde, telemetry::snapshot
use std::collections::VecDeque;

use serde::Serialize;

use super::snapshot::SystemSnapshot;

/// Ring buffer of optional samples; a gap (`None`) is kept so graphs show outages honestly.
#[derive(Debug, Clone, PartialEq)]
pub struct History {
    capacity: usize,
    samples: VecDeque<Option<f32>>,
}

impl History {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self { capacity, samples: VecDeque::with_capacity(capacity) }
    }

    pub fn push(&mut self, sample: Option<f32>) {
        if self.samples.len() == self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(sample.filter(|v| v.is_finite()));
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Samples oldest-first.
    pub fn values(&self) -> Vec<Option<f32>> {
        self.samples.iter().copied().collect()
    }

    /// The newest `n` samples, oldest-first.
    pub fn tail(&self, n: usize) -> Vec<Option<f32>> {
        let skip = self.samples.len().saturating_sub(n);
        self.samples.iter().skip(skip).copied().collect()
    }

    pub fn latest(&self) -> Option<f32> {
        self.samples.back().copied().flatten()
    }

    pub fn peak(&self) -> Option<f32> {
        self.samples.iter().flatten().copied().reduce(f32::max)
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }
}

/// Series kept for the tray icons and the monitoring window.
#[derive(Debug, Clone)]
pub struct MetricsHistory {
    pub timestamps: VecDeque<u64>,
    pub cpu_temp: History,
    pub gpu_temp: History,
    pub cpu_fan: History,
    pub gpu_fan: History,
    pub cpu_util: History,
    pub gpu_util: History,
    pub cpu_clock: History,
    pub gpu_clock: History,
    pub gpu_power: History,
    capacity: usize,
}

/// JSON shape sent to the monitoring view.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct HistoryView {
    pub timestamps: Vec<u64>,
    pub cpu_temp: Vec<Option<f32>>,
    pub gpu_temp: Vec<Option<f32>>,
    pub cpu_fan: Vec<Option<f32>>,
    pub gpu_fan: Vec<Option<f32>>,
    pub cpu_util: Vec<Option<f32>>,
    pub gpu_util: Vec<Option<f32>>,
    pub cpu_clock: Vec<Option<f32>>,
    pub gpu_clock: Vec<Option<f32>>,
    pub gpu_power: Vec<Option<f32>>,
}

impl MetricsHistory {
    pub fn new(capacity: usize) -> Self {
        let h = || History::new(capacity);
        Self {
            timestamps: VecDeque::with_capacity(capacity.max(1)),
            cpu_temp: h(),
            gpu_temp: h(),
            cpu_fan: h(),
            gpu_fan: h(),
            cpu_util: h(),
            gpu_util: h(),
            cpu_clock: h(),
            gpu_clock: h(),
            gpu_power: h(),
            capacity: capacity.max(1),
        }
    }

    pub fn push(&mut self, s: &SystemSnapshot) {
        if self.timestamps.len() == self.capacity {
            self.timestamps.pop_front();
        }
        self.timestamps.push_back(s.timestamp_ms);
        let rpm = |f: Option<crate::telemetry::snapshot::FanTelemetry>| f.and_then(|f| f.rpm).map(|r| r as f32);
        self.cpu_temp.push(s.cpu.temperature_c);
        self.gpu_temp.push(s.gpu.temperature_c);
        self.cpu_fan.push(rpm(s.cpu_fan));
        self.gpu_fan.push(rpm(s.gpu_fan));
        self.cpu_util.push(s.cpu.utilisation_pct);
        self.gpu_util.push(s.gpu.utilisation_pct);
        self.cpu_clock.push(s.cpu.frequency_mhz.map(|v| v as f32));
        self.gpu_clock.push(s.gpu.frequency_mhz.map(|v| v as f32));
        self.gpu_power.push(s.gpu.power_w);
    }

    pub fn len(&self) -> usize {
        self.timestamps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.timestamps.is_empty()
    }

    pub fn view(&self) -> HistoryView {
        HistoryView {
            timestamps: self.timestamps.iter().copied().collect(),
            cpu_temp: self.cpu_temp.values(),
            gpu_temp: self.gpu_temp.values(),
            cpu_fan: self.cpu_fan.values(),
            gpu_fan: self.gpu_fan.values(),
            cpu_util: self.cpu_util.values(),
            gpu_util: self.gpu_util.values(),
            cpu_clock: self.cpu_clock.values(),
            gpu_clock: self.gpu_clock.values(),
            gpu_power: self.gpu_power.values(),
        }
    }
}
