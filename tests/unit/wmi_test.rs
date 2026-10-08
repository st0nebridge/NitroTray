//! @module wmi_test
//! @description The real WMI transport: non-elevated callers get AccessDenied (observed on this machine);
//! elevated callers get the executor. Also checks HRESULT classification.
use nitrotray::acer::backend::AcerError;
use nitrotray::acer::gaming::WmiExecutor;
use nitrotray::acer::wmi::{classify, WmiGamingExecutor};

fn elevated() -> bool {
    // Registry key readable only by administrators is a cheap elevation probe.
    std::process::Command::new("net").arg("session").output().map(|o| o.status.success()).unwrap_or(false)
}

#[test]
fn connect_reflects_privilege_level() {
    match WmiGamingExecutor::connect() {
        Ok(mut exec) => {
            assert!(elevated(), "only an elevated caller can open AcerGamingFunction");
            let out = exec.get("GetGamingSysInfo", 0x0000).unwrap();
            assert_eq!(out & 0xFF, 0, "supported-sensors query succeeds");
        }
        Err(e) => {
            assert!(!elevated());
            assert!(matches!(e, AcerError::AccessDenied | AcerError::NotPresent), "{e:?}");
        }
    }
}

#[test]
fn classifies_hresults() {
    let err = |code: u32| windows::core::Error::from_hresult(windows::core::HRESULT(code as i32));
    assert_eq!(classify(&err(0x8004_1003)), AcerError::AccessDenied);
    assert_eq!(classify(&err(0x8007_0005)), AcerError::AccessDenied);
    assert_eq!(classify(&err(0x8004_1002)), AcerError::NotPresent);
    assert_eq!(classify(&err(0x8004_1010)), AcerError::NotPresent);
    assert_eq!(classify(&err(0x8004_100E)), AcerError::NotPresent);
    assert_eq!(classify(&err(0x8000_4005)), AcerError::Transport("HRESULT 0x80004005".into()));
}
