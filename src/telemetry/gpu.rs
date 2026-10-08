//! @module telemetry::gpu
//! @description NVIDIA GPU telemetry through NVML, loaded dynamically from the driver (no link-time dependency).
//!
//! @input  None; loads nvml.dll from System32 (or the NVSMI folder) on construction.
//! @output `GpuSample` (temperature, utilisation, graphics clock, board power).
//! @dependencies windows (LibraryLoader), telemetry (GpuSample, GpuSource)
//!
//! NVML can wake a sleeping discrete GPU, so the tray polls it only while a UI surface is
//! visible (DECISIONS D-20260930-010).
use super::{GpuSample, GpuSource};

/// NVML reports power in milliwatts.
pub fn milliwatts_to_watts(mw: u32) -> f32 {
    (mw as f32 / 100.0).round() / 10.0
}

#[cfg(windows)]
pub use nvml::Nvml;

#[cfg(windows)]
mod nvml {
    use std::ffi::c_void;

    use windows::core::{s, HSTRING};
    use windows::Win32::Foundation::{FreeLibrary, HMODULE};
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32};

    use super::{milliwatts_to_watts, GpuSample, GpuSource};

    type Device = *mut c_void;
    #[repr(C)]
    #[derive(Default)]
    struct Utilization {
        gpu: u32,
        memory: u32,
    }
    type FnVoid = unsafe extern "C" fn() -> i32;
    type FnHandle = unsafe extern "C" fn(u32, *mut Device) -> i32;
    type FnTemp = unsafe extern "C" fn(Device, u32, *mut u32) -> i32;
    type FnUtil = unsafe extern "C" fn(Device, *mut Utilization) -> i32;
    type FnClock = unsafe extern "C" fn(Device, u32, *mut u32) -> i32;
    type FnPower = unsafe extern "C" fn(Device, *mut u32) -> i32;

    const NVML_TEMPERATURE_GPU: u32 = 0;
    const NVML_CLOCK_GRAPHICS: u32 = 0;
    const NVSMI_PATH: &str = r"C:\Program Files\NVIDIA Corporation\NVSMI\nvml.dll";

    pub struct Nvml {
        lib: HMODULE,
        shutdown: FnVoid,
        temp: FnTemp,
        util: FnUtil,
        clock: FnClock,
        power: Option<FnPower>,
        device: Device,
    }

    // SAFETY: NVML is thread-safe; the handle is only used by its owner.
    unsafe impl Send for Nvml {}

    impl Nvml {
        pub fn load() -> Result<Self, String> {
            // SAFETY: symbols are transmuted to their documented NVML signatures.
            unsafe {
                let lib = LoadLibraryExW(&HSTRING::from("nvml.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32)
                    .or_else(|_| LoadLibraryExW(&HSTRING::from(NVSMI_PATH), None, Default::default()))
                    .map_err(|_| "nvml.dll not found (no NVIDIA driver)".to_string())?;
                macro_rules! sym {
                    ($name:expr, $ty:ty) => {
                        GetProcAddress(lib, $name).map(|f| std::mem::transmute::<_, $ty>(f))
                    };
                }
                let (Some(init), Some(shutdown), Some(handle), Some(temp), Some(util), Some(clock)) = (
                    sym!(s!("nvmlInit_v2"), FnVoid),
                    sym!(s!("nvmlShutdown"), FnVoid),
                    sym!(s!("nvmlDeviceGetHandleByIndex_v2"), FnHandle),
                    sym!(s!("nvmlDeviceGetTemperature"), FnTemp),
                    sym!(s!("nvmlDeviceGetUtilizationRates"), FnUtil),
                    sym!(s!("nvmlDeviceGetClockInfo"), FnClock),
                ) else {
                    let _ = FreeLibrary(lib);
                    return Err("nvml.dll is missing required entry points".into());
                };
                let power = sym!(s!("nvmlDeviceGetPowerUsage"), FnPower);
                if init() != 0 {
                    let _ = FreeLibrary(lib);
                    return Err("nvmlInit failed".into());
                }
                let mut device: Device = std::ptr::null_mut();
                if handle(0, &mut device) != 0 {
                    shutdown();
                    let _ = FreeLibrary(lib);
                    return Err("no NVIDIA GPU at index 0".into());
                }
                Ok(Self { lib, shutdown, temp, util, clock, power, device })
            }
        }
    }

    impl GpuSource for Nvml {
        fn sample(&mut self) -> Result<GpuSample, String> {
            // SAFETY: calls into NVML with a valid device handle and local out-params.
            unsafe {
                let mut t = 0u32;
                let temp = ((self.temp)(self.device, NVML_TEMPERATURE_GPU, &mut t) == 0).then_some(t as f32);
                let mut u = Utilization::default();
                let util = ((self.util)(self.device, &mut u) == 0).then_some(u.gpu.min(100) as f32);
                let mut c = 0u32;
                let mhz = ((self.clock)(self.device, NVML_CLOCK_GRAPHICS, &mut c) == 0).then_some(c);
                let mut mw = 0u32;
                let power = self.power.and_then(|f| (f(self.device, &mut mw) == 0).then(|| milliwatts_to_watts(mw)));
                if temp.is_none() && util.is_none() && mhz.is_none() {
                    return Err("NVML returned no readings".into());
                }
                Ok(GpuSample { temperature_c: temp, utilisation_pct: util, frequency_mhz: mhz, power_w: power })
            }
        }
    }

    impl Drop for Nvml {
        fn drop(&mut self) {
            // SAFETY: balanced with the successful init in `load`.
            unsafe {
                (self.shutdown)();
                let _ = FreeLibrary(self.lib);
            }
        }
    }
}

/// Loads NVML on first use only, so an app that never shows a window never touches the GPU driver.
#[cfg(windows)]
#[derive(Default)]
pub struct LazyNvml {
    inner: Option<Nvml>,
    failure: Option<String>,
}

#[cfg(windows)]
impl GpuSource for LazyNvml {
    fn sample(&mut self) -> Result<GpuSample, String> {
        if let Some(reason) = &self.failure {
            return Err(reason.clone());
        }
        if self.inner.is_none() {
            match Nvml::load() {
                Ok(n) => self.inner = Some(n),
                Err(e) => {
                    self.failure = Some(e.clone());
                    return Err(e);
                }
            }
        }
        self.inner.as_mut().map_or_else(|| Err("NVML unavailable".into()), GpuSource::sample)
    }
}
