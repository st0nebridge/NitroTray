//! @module acer::wmi
//! @description COM/WMI transport for AcerGamingFunction (root\WMI): executes Get*/Set* gaming methods.
//!
//! @input  Method name and `gmInput` integer.
//! @output Raw `gmOutput` integer, or an `AcerError` (AccessDenied when not elevated, NotPresent off-Acer).
//! @dependencies windows (COM, WMI, Variant), acer::backend, acer::gaming (WmiExecutor)
//!
//! Must be created and used on a single thread (COM objects are not `Send`); the helper owns it
//! on its dedicated hardware thread. uint32 inputs travel as VT_I4, uint64 as VT_BSTR (WMI rule).
use std::mem::ManuallyDrop;

use windows::core::{BSTR, HRESULT, PCWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoSetProxyBlanket, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, EOAC_NONE,
    RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::Variant::{VariantClear, VariantToUInt64, VARIANT, VT_BSTR, VT_I4};
use windows::Win32::System::Wmi::{
    IWbemClassObject, IWbemLocator, IWbemServices, WbemLocator, WBEM_FLAG_FORWARD_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY,
    WBEM_FLAG_RETURN_WBEM_COMPLETE, WBEM_INFINITE,
};

use crate::acer::backend::AcerError;
use crate::acer::gaming::WmiExecutor;

const CLASS: &str = "AcerGamingFunction";
const WBEM_E_ACCESS_DENIED: HRESULT = HRESULT(0x8004_1003_u32 as i32);
const E_ACCESSDENIED: HRESULT = HRESULT(0x8007_0005_u32 as i32);
const WBEM_E_NOT_FOUND: HRESULT = HRESULT(0x8004_1002_u32 as i32);
const WBEM_E_INVALID_CLASS: HRESULT = HRESULT(0x8004_1010_u32 as i32);
const WBEM_E_INVALID_NAMESPACE: HRESULT = HRESULT(0x8004_100E_u32 as i32);

/// Maps a COM error to the backend error vocabulary.
pub fn classify(err: &windows::core::Error) -> AcerError {
    match err.code() {
        c if c == WBEM_E_ACCESS_DENIED || c == E_ACCESSDENIED => AcerError::AccessDenied,
        c if c == WBEM_E_NOT_FOUND || c == WBEM_E_INVALID_CLASS || c == WBEM_E_INVALID_NAMESPACE => {
            AcerError::NotPresent
        }
        c => AcerError::Transport(format!("HRESULT 0x{:08X}", c.0 as u32)),
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn variant_i4(v: i32) -> VARIANT {
    let mut var = VARIANT::default();
    // SAFETY: writing the discriminant and the matching union member of a zeroed VARIANT.
    unsafe {
        (*var.Anonymous.Anonymous).vt = VT_I4;
        (*var.Anonymous.Anonymous).Anonymous.lVal = v;
    }
    var
}

fn variant_bstr(s: &str) -> VARIANT {
    let mut var = VARIANT::default();
    // SAFETY: as above; ownership of the BSTR moves into the VARIANT and is freed by VariantClear.
    unsafe {
        (*var.Anonymous.Anonymous).vt = VT_BSTR;
        (*var.Anonymous.Anonymous).Anonymous.bstrVal = ManuallyDrop::new(BSTR::from(s));
    }
    var
}

/// Reads an integer out of a VARIANT, accepting WMI's BSTR encoding of 64-bit values.
fn variant_u64(var: &VARIANT) -> Result<u64, AcerError> {
    // SAFETY: the discriminant is read before the matching member.
    unsafe {
        let inner = &*var.Anonymous.Anonymous;
        if inner.vt == VT_BSTR {
            let s = inner.Anonymous.bstrVal.to_string();
            return s.trim().parse::<u64>().map_err(|_| AcerError::Transport(format!("bad uint64 '{s}'")));
        }
        if inner.vt == VT_I4 {
            return Ok(u64::from(inner.Anonymous.lVal as u32));
        }
        VariantToUInt64(var).map_err(|e| AcerError::Transport(format!("gmOutput: {e}")))
    }
}

pub struct WmiGamingExecutor {
    services: IWbemServices,
    class: IWbemClassObject,
    instance_path: BSTR,
}

impl WmiGamingExecutor {
    /// Connects to root\WMI and locates the AcerGamingFunction instance on the calling thread.
    pub fn connect() -> Result<Self, AcerError> {
        // SAFETY: plain COM initialisation and calls with owned arguments.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let locator: IWbemLocator =
                CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER).map_err(|e| classify(&e))?;
            let services = locator
                .ConnectServer(
                    &BSTR::from("root\\WMI"),
                    &BSTR::new(),
                    &BSTR::new(),
                    &BSTR::new(),
                    0,
                    &BSTR::new(),
                    None,
                )
                .map_err(|e| classify(&e))?;
            CoSetProxyBlanket(
                &services,
                10,
                0,
                PCWSTR::null(),
                RPC_C_AUTHN_LEVEL_CALL,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                EOAC_NONE,
            )
            .map_err(|e| classify(&e))?;
            let mut class = None;
            services
                .GetObject(&BSTR::from(CLASS), WBEM_FLAG_RETURN_WBEM_COMPLETE, None, Some(&mut class), None)
                .map_err(|e| classify(&e))?;
            let class = class.ok_or(AcerError::NotPresent)?;
            let instance_path = Self::find_instance(&services)?;
            Ok(Self { services, class, instance_path })
        }
    }

    unsafe fn find_instance(services: &IWbemServices) -> Result<BSTR, AcerError> {
        let query = format!("SELECT __RELPATH FROM {CLASS}");
        let rows = services
            .ExecQuery(
                &BSTR::from("WQL"),
                &BSTR::from(query.as_str()),
                WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY,
                None,
            )
            .map_err(|e| classify(&e))?;
        let mut row = [None];
        let mut returned = 0u32;
        let hr = rows.Next(WBEM_INFINITE, &mut row, &mut returned);
        if hr.is_err() {
            return Err(classify(&windows::core::Error::from(hr)));
        }
        let obj = row[0].take().filter(|_| returned == 1).ok_or(AcerError::NotPresent)?;
        let mut var = VARIANT::default();
        let key = wide("__RELPATH");
        obj.Get(PCWSTR(key.as_ptr()), 0, &mut var, None, None).map_err(|e| classify(&e))?;
        let path = if (*var.Anonymous.Anonymous).vt == VT_BSTR {
            (*(*var.Anonymous.Anonymous).Anonymous.bstrVal).clone()
        } else {
            BSTR::new()
        };
        let _ = VariantClear(&mut var);
        if path.is_empty() {
            Err(AcerError::NotPresent)
        } else {
            Ok(path)
        }
    }

    fn call(&mut self, method: &str, mut input: VARIANT) -> Result<u64, AcerError> {
        // SAFETY: COM calls on objects owned by this thread; VARIANTs are cleared on every path.
        unsafe {
            let name = wide(method);
            let mut in_sig = None;
            let res = self.class.GetMethod(PCWSTR(name.as_ptr()), 0, &mut in_sig, std::ptr::null_mut());
            let run = || -> Result<u64, AcerError> {
                res.map_err(|e| classify(&e))?;
                let params = in_sig.ok_or(AcerError::NotPresent)?.SpawnInstance(0).map_err(|e| classify(&e))?;
                let arg = wide("gmInput");
                params.Put(PCWSTR(arg.as_ptr()), 0, &input, 0).map_err(|e| classify(&e))?;
                let mut out = None;
                self.services
                    .ExecMethod(
                        &self.instance_path,
                        &BSTR::from(method),
                        WBEM_FLAG_RETURN_WBEM_COMPLETE,
                        None,
                        &params,
                        Some(&mut out),
                        None,
                    )
                    .map_err(|e| classify(&e))?;
                let out = out.ok_or_else(|| AcerError::Transport("no output parameters".into()))?;
                let mut value = VARIANT::default();
                let key = wide("gmOutput");
                out.Get(PCWSTR(key.as_ptr()), 0, &mut value, None, None).map_err(|e| classify(&e))?;
                let result = variant_u64(&value);
                let _ = VariantClear(&mut value);
                result
            };
            let result = run();
            let _ = VariantClear(&mut input);
            result
        }
    }
}

impl WmiExecutor for WmiGamingExecutor {
    fn get(&mut self, method: &str, input: u32) -> Result<u64, AcerError> {
        self.call(method, variant_i4(input as i32))
    }

    fn set(&mut self, method: &str, input: u64) -> Result<u32, AcerError> {
        self.call(method, variant_bstr(&input.to_string())).map(|v| v as u32)
    }
}
