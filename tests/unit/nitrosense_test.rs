//! @module nitrosense_test
//! @description NitroSense AUMID validation and launch-target construction (no launch in tests).
use nitrotray::acer::nitrosense::{is_valid_aumid, launch_target, DEFAULT_AUMID};

#[test]
fn default_aumid_is_valid() {
    assert!(is_valid_aumid(DEFAULT_AUMID));
    assert_eq!(
        launch_target(DEFAULT_AUMID).as_deref(),
        Some(r"shell:AppsFolder\AcerIncorporated.NitroSenseV31_48frkmn4z8aw4!App")
    );
}

#[test]
fn rejects_injection_and_malformed_ids() {
    for bad in ["", "NoBang", "a!b!c", "!App", "Pkg!", "Pkg!App & calc", r"..\..\x!App", "Pkg!Ap p"] {
        assert!(!is_valid_aumid(bad), "{bad}");
        assert_eq!(launch_target(bad), None);
    }
    assert!(!is_valid_aumid(&format!("{}!App", "a".repeat(201))));
}

#[test]
fn launch_refuses_invalid_ids_without_side_effects() {
    let err = nitrotray::acer::nitrosense::launch("calc.exe & whoami").unwrap_err();
    assert!(err.contains("invalid NitroSense app id"));
}
