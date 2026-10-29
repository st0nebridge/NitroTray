//! @module unlock_boot_log_test
//! @description The UEFI unlocker's boot log: which facts say a boot unlocked the GPU, and the run name rule.
use nitrotray::unlock::boot_log::{is_run_log_name, parse, BootStatus};

const UNLOCKED: &str = "RTX 4060 Laptop unlock build=auto profile=entryLatchedSs1Full\n\
banner banner.txt: status=0\n\
Secure Boot on: started through shim (MOK-verified image)\n\
Checks passed.\n\
choice: unlock\n\
POST-GOP SS0=0x88888888 SS1=0x8\n\
TRANSACTION OK: hardware readbacks passed, DMA released, display driver reconnected\n\
Unlocked. Starting Windows.\n";

fn without(text: &str, line_start: &str) -> String {
    text.lines().filter(|l| !l.starts_with(line_start)).collect::<Vec<_>>().join("\n")
}

#[test]
fn a_transaction_with_the_unlocked_selectors_unlocked_the_boot() {
    let b = parse("RUN0028.TXT", UNLOCKED, Some(1_793_000_000));
    assert_eq!(b.status, BootStatus::Unlocked);
    assert_eq!(b.run, "RUN0028");
    assert_eq!(b.build, "auto");
    assert_eq!(b.written_unix, Some(1_793_000_000));
}

#[test]
fn the_selectors_must_match_exactly() {
    for (ss0, ss1) in [("0x88888888", "0x5"), ("0x2707044", "0x8"), ("0x88888880", "0x8")] {
        let text = UNLOCKED.replace("SS0=0x88888888 SS1=0x8", &format!("SS0={ss0} SS1={ss1}"));
        assert_eq!(parse("RUN0001.TXT", &text, None).status, BootStatus::NotUnlocked, "{ss0} {ss1}");
    }
    let padded = UNLOCKED.replace("SS1=0x8", "SS1=0x08");
    assert_eq!(parse("RUN0001.TXT", &padded, None).status, BootStatus::Unlocked, "leading zeros are the same number");
}

#[test]
fn a_transaction_without_a_readback_or_the_readback_without_a_transaction_is_not_unlocked() {
    assert_eq!(parse("RUN0001.TXT", &without(UNLOCKED, "TRANSACTION OK"), None).status, BootStatus::NotUnlocked);
    assert_eq!(parse("RUN0001.TXT", &without(UNLOCKED, "POST-GOP"), None).status, BootStatus::NotUnlocked);
}

#[test]
fn a_stop_beats_everything_else() {
    let stopped = format!("{UNLOCKED}STOP: stage=display-release\n");
    assert_eq!(parse("RUN0016.TXT", &stopped, None).status, BootStatus::Stopped);
    let console = format!("{UNLOCKED}STOPPED\n");
    assert_eq!(parse("RUN0016.TXT", &console, None).status, BootStatus::Stopped);
}

#[test]
fn a_skip_is_not_an_unlock_and_not_a_dry_run() {
    let text = "RTX 4060 Laptop unlock build=dryrun profile=x\nSKIP: stage=cold-state\n";
    assert_eq!(parse("RUN0019.TXT", text, None).status, BootStatus::Skipped);
}

#[test]
fn dry_run_builds_write_nothing() {
    for build in ["dryrun", "reltest"] {
        let text = format!("RTX 4060 Laptop unlock build={build} profile=x\nChecks passed.\n");
        assert_eq!(parse("RUN0020.TXT", &text, None).status, BootStatus::DryRun, "{build}");
    }
    let auto = "RTX 4060 Laptop unlock build=auto profile=x\nChecks passed.\n";
    assert_eq!(parse("RUN0020.TXT", auto, None).status, BootStatus::NotUnlocked, "an auto build that never wrote");
}

#[test]
fn a_missing_build_is_empty_not_a_guess() {
    let b = parse("RUN0002.TXT", "nothing useful\n", None);
    assert_eq!((b.status, b.build.as_str()), (BootStatus::NotUnlocked, ""));
    assert_eq!(b.written_unix, None);
}

#[test]
fn only_run_logs_are_read() {
    for ok in ["RUN0001.TXT", "RUN0028.TXT", "run0028.txt", "RUN12345.TXT"] {
        assert!(is_run_log_name(ok), "{ok}");
    }
    for bad in
        ["RUN.TXT", "RUN0028.TXT.bak", "XRUN0028.TXT", "RUN00A8.TXT", "banner.txt", "RUN0028", "RUN0028.TXT\\", ""]
    {
        assert!(!is_run_log_name(bad), "{bad}");
    }
}

#[test]
fn the_run_name_drops_the_extension_in_any_case() {
    assert_eq!(parse("run0007.txt", UNLOCKED, None).run, "run0007");
    assert_eq!(parse("RUN0007", UNLOCKED, None).run, "RUN0007");
}

#[test]
fn build_is_the_first_key_that_starts_a_word() {
    for text in ["rebuild=junk\nRTX build=auto\n", "_build=junk RTX build=auto", "9build=junk build=auto"] {
        assert_eq!(parse("RUN0001.TXT", text, None).build, "auto", "{text:?}");
    }
    assert_eq!(parse("RUN0001.TXT", "build=auto2_x-y", None).build, "auto2_x", "the value is one word");
    assert_eq!(parse("RUN0001.TXT", "(build=auto)", None).build, "auto", "punctuation is not part of a word");
    assert_eq!(parse("RUN0001.TXT", "build=", None).build, "");
}
