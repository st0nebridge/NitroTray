//! @module telemetry::cpu
//! @description CPU utilisation and effective clock via PDH (same counters Task Manager uses).
//!
//! @input  None; opens a PDH query on construction.
//! @output `CpuSample` (utility %, effective MHz = base MHz × % processor performance / 100).
//! @dependencies windows (System::Performance), telemetry (CpuSample, CpuSource)
use super::{CpuSample, CpuSource};

/// Effective MHz from the base frequency and the "% Processor Performance" counter.
pub fn effective_mhz(base_mhz: f64, performance_pct: f64) -> Option<u32> {
    (base_mhz.is_finite() && performance_pct.is_finite() && base_mhz > 0.0 && performance_pct > 0.0)
        .then(|| (base_mhz * performance_pct / 100.0).round() as u32)
}

/// Utility can exceed 100 % on turbo-capable parts; the popup shows 0–100.
pub fn clamp_pct(v: f64) -> Option<f32> {
    v.is_finite().then(|| v.clamp(0.0, 100.0) as f32)
}

#[cfg(windows)]
pub use pdh::PdhCpu;

#[cfg(windows)]
mod pdh {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterValue, PdhOpenQueryW,
        PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY,
    };

    use super::{clamp_pct, effective_mhz, CpuSample, CpuSource};

    const UTILITY: &str = r"\Processor Information(_Total)\% Processor Utility";
    const PERFORMANCE: &str = r"\Processor Information(_Total)\% Processor Performance";
    const FREQUENCY: &str = r"\Processor Information(_Total)\Processor Frequency";

    pub struct PdhCpu {
        query: PDH_HQUERY,
        utility: PDH_HCOUNTER,
        performance: PDH_HCOUNTER,
        frequency: PDH_HCOUNTER,
    }

    // SAFETY: PDH handles are process-wide and used by one owner at a time.
    unsafe impl Send for PdhCpu {}

    impl PdhCpu {
        pub fn open() -> Result<Self, String> {
            // SAFETY: PDH calls with out-pointers to locals; the query is closed on drop.
            unsafe {
                let mut query = PDH_HQUERY::default();
                let rc = PdhOpenQueryW(PCWSTR::null(), 0, &mut query);
                if rc != 0 {
                    return Err(format!("PdhOpenQuery failed: 0x{rc:08X}"));
                }
                let mut me = Self {
                    query,
                    utility: Default::default(),
                    performance: Default::default(),
                    frequency: Default::default(),
                };
                for (path, slot) in
                    [(UTILITY, &mut me.utility), (PERFORMANCE, &mut me.performance), (FREQUENCY, &mut me.frequency)]
                {
                    let rc = PdhAddEnglishCounterW(me.query, &HSTRING::from(path), 0, slot);
                    if rc != 0 {
                        return Err(format!("counter {path} unavailable: 0x{rc:08X}"));
                    }
                }
                PdhCollectQueryData(me.query);
                Ok(me)
            }
        }

        fn value(counter: PDH_HCOUNTER) -> Option<f64> {
            let mut v = PDH_FMT_COUNTERVALUE::default();
            // SAFETY: reads a formatted value into a local.
            let rc = unsafe { PdhGetFormattedCounterValue(counter, PDH_FMT_DOUBLE, None, &mut v) };
            // SAFETY: PDH_FMT_DOUBLE fills the doubleValue member; the union is plain data.
            (rc == 0 && v.CStatus <= 1).then_some(unsafe { v.Anonymous.doubleValue })
        }
    }

    impl CpuSource for PdhCpu {
        fn sample(&mut self) -> Result<CpuSample, String> {
            // SAFETY: collects on the query this value owns.
            let rc = unsafe { PdhCollectQueryData(self.query) };
            if rc != 0 {
                return Err(format!("PdhCollectQueryData failed: 0x{rc:08X}"));
            }
            let util = Self::value(self.utility).and_then(clamp_pct);
            let mhz = match (Self::value(self.frequency), Self::value(self.performance)) {
                (Some(base), Some(perf)) => effective_mhz(base, perf),
                _ => None,
            };
            Ok(CpuSample { utilisation_pct: util, frequency_mhz: mhz })
        }
    }

    impl Drop for PdhCpu {
        fn drop(&mut self) {
            // SAFETY: closes the query opened in `open`.
            unsafe {
                PdhCloseQuery(self.query);
            }
        }
    }
}
