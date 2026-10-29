//! @module unlock::cuda
//! @description Runs the tensor probe on the GPU through the CUDA driver API (`nvcuda.dll`, loaded from
//! System32 at the moment of use; no toolkit, no link-time dependency). It creates a context, times the
//! two kernels from `unlock::ptx`, and destroys everything again, so nothing stays on the GPU afterwards.
//!
//! @input  Nothing; the first NVIDIA GPU is measured.
//! @output `TensorRates`, or why the GPU could not be measured.
//! @dependencies windows (LibraryLoader), unlock::{ptx, state}
//!
//! This wakes a sleeping discrete GPU like any other use of it. The app calls it only while a UI surface
//! that shows the result is open or when asked (D-20260930-010).
use std::ffi::{c_char, c_void, CString};

use windows::core::{s, HSTRING};
use windows::Win32::Foundation::{FreeLibrary, HMODULE};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32};

use super::ptx::{self, Accumulate};
use super::state::TensorRates;

type Handle = *mut c_void;
type FnInit = unsafe extern "C" fn(u32) -> i32;
type FnDeviceGet = unsafe extern "C" fn(*mut i32, i32) -> i32;
type FnAttribute = unsafe extern "C" fn(*mut i32, i32, i32) -> i32;
type FnCtxCreate = unsafe extern "C" fn(*mut Handle, u32, i32) -> i32;
type FnHandle = unsafe extern "C" fn(Handle) -> i32;
type FnAlloc = unsafe extern "C" fn(*mut u64, usize) -> i32;
type FnFree = unsafe extern "C" fn(u64) -> i32;
type FnModuleLoad = unsafe extern "C" fn(*mut Handle, *const c_void) -> i32;
type FnGetFunction = unsafe extern "C" fn(*mut Handle, Handle, *const c_char) -> i32;
type FnLaunch =
    unsafe extern "C" fn(Handle, u32, u32, u32, u32, u32, u32, u32, Handle, *mut *mut c_void, *mut *mut c_void) -> i32;
type FnEventCreate = unsafe extern "C" fn(*mut Handle, u32) -> i32;
type FnEventRecord = unsafe extern "C" fn(Handle, Handle) -> i32;
type FnElapsed = unsafe extern "C" fn(*mut f32, Handle, Handle) -> i32;

const ATTR_MULTIPROCESSOR_COUNT: i32 = 16;
const ATTR_COMPUTE_CAPABILITY_MAJOR: i32 = 75;
/// `mma.sync.m16n8k16` needs compute capability 8.0 (Ampere) or newer.
const MIN_MAJOR: i32 = 8;
const OUT_BYTES: usize = 4096;

struct Api {
    lib: HMODULE,
    init: FnInit,
    device_get: FnDeviceGet,
    attribute: FnAttribute,
    ctx_create: FnCtxCreate,
    ctx_destroy: FnHandle,
    alloc: FnAlloc,
    free: FnFree,
    module_load: FnModuleLoad,
    module_unload: FnHandle,
    get_function: FnGetFunction,
    launch: FnLaunch,
    event_create: FnEventCreate,
    event_record: FnEventRecord,
    event_sync: FnHandle,
    elapsed: FnElapsed,
    event_destroy: FnHandle,
}

impl Api {
    fn load() -> Result<Self, String> {
        // SAFETY: symbols are transmuted to their documented CUDA driver API signatures.
        unsafe {
            let lib = LoadLibraryExW(&HSTRING::from("nvcuda.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32)
                .map_err(|_| "nvcuda.dll not found (no NVIDIA driver)".to_string())?;
            macro_rules! sym {
                ($name:expr, $ty:ty) => {
                    GetProcAddress(lib, $name).map(|f| std::mem::transmute::<_, $ty>(f))
                };
            }
            let api = (|| {
                Some(Self {
                    lib,
                    init: sym!(s!("cuInit"), FnInit)?,
                    device_get: sym!(s!("cuDeviceGet"), FnDeviceGet)?,
                    attribute: sym!(s!("cuDeviceGetAttribute"), FnAttribute)?,
                    ctx_create: sym!(s!("cuCtxCreate_v2"), FnCtxCreate)?,
                    ctx_destroy: sym!(s!("cuCtxDestroy_v2"), FnHandle)?,
                    alloc: sym!(s!("cuMemAlloc_v2"), FnAlloc)?,
                    free: sym!(s!("cuMemFree_v2"), FnFree)?,
                    module_load: sym!(s!("cuModuleLoadData"), FnModuleLoad)?,
                    module_unload: sym!(s!("cuModuleUnload"), FnHandle)?,
                    get_function: sym!(s!("cuModuleGetFunction"), FnGetFunction)?,
                    launch: sym!(s!("cuLaunchKernel"), FnLaunch)?,
                    event_create: sym!(s!("cuEventCreate"), FnEventCreate)?,
                    event_record: sym!(s!("cuEventRecord"), FnEventRecord)?,
                    event_sync: sym!(s!("cuEventSynchronize"), FnHandle)?,
                    elapsed: sym!(s!("cuEventElapsedTime"), FnElapsed)?,
                    event_destroy: sym!(s!("cuEventDestroy_v2"), FnHandle)?,
                })
            })();
            api.ok_or_else(|| {
                let _ = FreeLibrary(lib);
                "nvcuda.dll is missing required entry points".to_string()
            })
        }
    }
}

impl Drop for Api {
    fn drop(&mut self) {
        // SAFETY: balanced with the load in `Api::load`; every handle made through it is gone first.
        unsafe {
            let _ = FreeLibrary(self.lib);
        }
    }
}

fn check(code: i32, what: &str) -> Result<(), String> {
    if code == 0 {
        Ok(())
    } else {
        Err(format!("{what} failed (CUDA error {code})"))
    }
}

/// Destroys a CUDA handle with its destroy call when it goes out of scope.
struct Owned<'a> {
    handle: Handle,
    destroy: FnHandle,
    _api: &'a Api,
}

impl<'a> Owned<'a> {
    fn new(api: &'a Api, handle: Handle, destroy: FnHandle) -> Self {
        Self { handle, destroy, _api: api }
    }
}

impl Drop for Owned<'_> {
    fn drop(&mut self) {
        // SAFETY: the handle came from the matching create call and is destroyed once.
        unsafe {
            (self.destroy)(self.handle);
        }
    }
}

struct Buffer<'a> {
    api: &'a Api,
    ptr: u64,
}

impl Drop for Buffer<'_> {
    fn drop(&mut self) {
        // SAFETY: the allocation came from `cuMemAlloc_v2` and is freed once, inside its context.
        unsafe {
            (self.api.free)(self.ptr);
        }
    }
}

/// Best throughput of `ROUNDS` timed launches of one kernel, in TFLOPS.
///
/// # Safety
/// `ctx` must be the current context and `out` a live allocation of at least `OUT_BYTES`.
unsafe fn time_kernel(api: &Api, acc: Accumulate, blocks: usize, out: u64) -> Result<f32, String> {
    let ptx = CString::new(ptx::kernel(acc)).map_err(|_| "the probe's PTX holds a NUL".to_string())?;
    let mut module: Handle = std::ptr::null_mut();
    check((api.module_load)(&mut module, ptx.as_ptr().cast()), "loading the probe kernel")?;
    let module = Owned::new(api, module, api.module_unload);
    let name = CString::new(ptx::ENTRY).map_err(|_| "bad entry name".to_string())?;
    let mut function: Handle = std::ptr::null_mut();
    check((api.get_function)(&mut function, module.handle, name.as_ptr()), "finding the probe kernel")?;
    let (mut start, mut end): (Handle, Handle) = (std::ptr::null_mut(), std::ptr::null_mut());
    check((api.event_create)(&mut start, 0), "creating an event")?;
    let start = Owned::new(api, start, api.event_destroy);
    check((api.event_create)(&mut end, 0), "creating an event")?;
    let end = Owned::new(api, end, api.event_destroy);

    let launch = |iters: u32| -> Result<(), String> {
        let (mut p_out, mut p_iters) = (out, iters);
        let mut params: [*mut c_void; 2] = [(&mut p_out as *mut u64).cast(), (&mut p_iters as *mut u32).cast()];
        check(
            (api.launch)(
                function,
                blocks as u32,
                1,
                1,
                ptx::THREADS as u32,
                1,
                1,
                0,
                std::ptr::null_mut(),
                params.as_mut_ptr(),
                std::ptr::null_mut(),
            ),
            "launching the probe kernel",
        )
    };

    launch(ptx::WARMUP_ITERS)?;
    let mut best = 0.0f32;
    for _ in 0..ptx::ROUNDS {
        check((api.event_record)(start.handle, std::ptr::null_mut()), "recording an event")?;
        launch(ptx::ITERS)?;
        check((api.event_record)(end.handle, std::ptr::null_mut()), "recording an event")?;
        check((api.event_sync)(end.handle), "waiting for the probe kernel")?;
        let mut ms = 0.0f32;
        check((api.elapsed)(&mut ms, start.handle, end.handle), "reading an event")?;
        if ms > 0.0 {
            best = best.max((ptx::flops(blocks, ptx::ITERS as usize) / (f64::from(ms) / 1000.0) / 1e12) as f32);
        }
    }
    Ok(best)
}

/// Times FP16 multiply-accumulate with FP32 and with FP16 accumulate on the first NVIDIA GPU.
pub fn measure() -> Result<TensorRates, String> {
    let api = Api::load()?;
    // SAFETY: plain driver calls on handles created here and released by their guards, in reverse order.
    unsafe {
        check((api.init)(0), "cuInit")?;
        let mut device = 0i32;
        check((api.device_get)(&mut device, 0), "finding the GPU")?;
        let (mut sms, mut major) = (0i32, 0i32);
        check((api.attribute)(&mut sms, ATTR_MULTIPROCESSOR_COUNT, device), "reading the SM count")?;
        check((api.attribute)(&mut major, ATTR_COMPUTE_CAPABILITY_MAJOR, device), "reading the compute capability")?;
        if major < MIN_MAJOR {
            return Err(format!(
                "the tensor probe needs compute capability {MIN_MAJOR}.0 or newer, this GPU has {major}.x"
            ));
        }
        let mut ctx: Handle = std::ptr::null_mut();
        check((api.ctx_create)(&mut ctx, 0, device), "creating a CUDA context")?;
        let _ctx = Owned::new(&api, ctx, api.ctx_destroy);
        let mut ptr = 0u64;
        check((api.alloc)(&mut ptr, OUT_BYTES), "allocating GPU memory")?;
        let out = Buffer { api: &api, ptr };
        let blocks = sms.max(1) as usize * ptx::BLOCKS_PER_SM;
        let acc32 = time_kernel(&api, Accumulate::F32, blocks, out.ptr)?;
        let acc16 = time_kernel(&api, Accumulate::F16, blocks, out.ptr)?;
        Ok(TensorRates { acc32_tflops: acc32, acc16_tflops: acc16 })
    }
}
