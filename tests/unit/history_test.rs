//! @module history_test
//! @description Ring-buffer histories used by the graph icons and the monitoring view.
use nitrotray::telemetry::history::{History, MetricsHistory};
use nitrotray::telemetry::snapshot::{CpuTelemetry, FanTelemetry, GpuTelemetry, SystemSnapshot};

#[test]
fn keeps_newest_samples_up_to_capacity() {
    let mut h = History::new(3);
    assert!(h.is_empty());
    for v in [1.0, 2.0, 3.0, 4.0] {
        h.push(Some(v));
    }
    assert_eq!(h.len(), 3);
    assert_eq!(h.capacity(), 3);
    assert_eq!(h.values(), vec![Some(2.0), Some(3.0), Some(4.0)]);
    assert_eq!(h.latest(), Some(4.0));
    assert_eq!(h.tail(2), vec![Some(3.0), Some(4.0)]);
    assert_eq!(h.tail(10).len(), 3);
}

#[test]
fn gaps_are_kept_and_non_finite_values_become_gaps() {
    let mut h = History::new(4);
    h.push(Some(5.0));
    h.push(None);
    h.push(Some(f32::NAN));
    h.push(Some(9.0));
    assert_eq!(h.values(), vec![Some(5.0), None, None, Some(9.0)]);
    assert_eq!(h.peak(), Some(9.0));
    h.push(None);
    assert_eq!(h.latest(), None);
    h.clear();
    assert!(h.is_empty());
    assert_eq!(h.peak(), None);
}

#[test]
fn zero_capacity_is_raised_to_one() {
    let mut h = History::new(0);
    h.push(Some(1.0));
    h.push(Some(2.0));
    assert_eq!(h.values(), vec![Some(2.0)]);
}

#[test]
fn metrics_history_splits_a_snapshot_into_series() {
    let mut m = MetricsHistory::new(2);
    assert!(m.is_empty());
    let snap = |t: u64| SystemSnapshot {
        timestamp_ms: t,
        cpu: CpuTelemetry { temperature_c: Some(80.0), utilisation_pct: Some(20.0), frequency_mhz: Some(3900) },
        gpu: GpuTelemetry {
            temperature_c: Some(70.0),
            utilisation_pct: Some(90.0),
            frequency_mhz: Some(2500),
            power_w: Some(95.5),
        },
        cpu_fan: Some(FanTelemetry { rpm: Some(7000), duty_pct: None }),
        gpu_fan: None,
        ..SystemSnapshot::default()
    };
    for t in [1, 2, 3] {
        m.push(&snap(t));
    }
    assert_eq!(m.len(), 2);
    let v = m.view();
    assert_eq!(v.timestamps, vec![2, 3]);
    assert_eq!(v.cpu_temp, vec![Some(80.0); 2]);
    assert_eq!(v.gpu_temp, vec![Some(70.0); 2]);
    assert_eq!(v.cpu_fan, vec![Some(7000.0); 2]);
    assert_eq!(v.gpu_fan, vec![None; 2]);
    assert_eq!(v.cpu_util, vec![Some(20.0); 2]);
    assert_eq!(v.gpu_util, vec![Some(90.0); 2]);
    assert_eq!(v.cpu_clock, vec![Some(3900.0); 2]);
    assert_eq!(v.gpu_clock, vec![Some(2500.0); 2]);
    assert_eq!(v.gpu_power, vec![Some(95.5); 2]);
}
