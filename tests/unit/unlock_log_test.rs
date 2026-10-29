//! @module unlock_log_test
//! @description Reading the unlocker's newest boot log from its directory: only RUN logs count, the highest
//! number wins, a missing directory is "no log", and only the typed report leaves the reader.
use std::path::{Path, PathBuf};

use nitrotray::service::unlock_log::{latest_in_dir, MAX_LOG_BYTES};
use nitrotray::unlock::boot_log::BootStatus;

const UNLOCKED: &str = "RTX 4060 Laptop unlock build=auto profile=x\nchoice: unlock\n\
POST-GOP SS0=0x88888888 SS1=0x8\nTRANSACTION OK: done\n";
const SKIPPED: &str = "RTX 4060 Laptop unlock build=auto profile=x\nSKIP: stage=cold-state\n";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nitrotray-unlock-log-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn put(dir: &Path, name: &str, text: &str) {
    std::fs::write(dir.join(name), text).unwrap();
}

#[test]
fn the_highest_run_number_is_the_newest() {
    let dir = scratch("highest");
    put(&dir, "RUN0009.TXT", UNLOCKED);
    put(&dir, "RUN0010.TXT", SKIPPED);
    put(&dir, "RUN0002.TXT", UNLOCKED);
    let report = latest_in_dir(&dir).unwrap().unwrap();
    assert_eq!((report.run.as_str(), report.status), ("RUN0010", BootStatus::Skipped), "10 is after 9, not before");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn only_run_logs_are_considered() {
    let dir = scratch("only-run");
    put(&dir, "RUN0001.TXT", UNLOCKED);
    put(&dir, "RUN0099.TXT.bak", SKIPPED);
    put(&dir, "banner.txt", SKIPPED);
    put(&dir, "RUNX.TXT", SKIPPED);
    std::fs::create_dir_all(dir.join("RUN0100.TXT")).unwrap();
    let report = latest_in_dir(&dir).unwrap().unwrap();
    assert_eq!((report.run.as_str(), report.status), ("RUN0001", BootStatus::Unlocked));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn no_directory_or_no_logs_is_no_report() {
    let missing = std::env::temp_dir().join(format!("nitrotray-unlock-log-missing-{}", std::process::id()));
    assert_eq!(latest_in_dir(&missing), Ok(None));
    let empty = scratch("empty");
    assert_eq!(latest_in_dir(&empty), Ok(None));
    let _ = std::fs::remove_dir_all(empty);
}

#[test]
fn the_write_time_comes_from_the_file() {
    let dir = scratch("time");
    put(&dir, "RUN0001.TXT", UNLOCKED);
    let written = latest_in_dir(&dir).unwrap().unwrap().written_unix.expect("a write time");
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    assert!(now.abs_diff(written) < 120, "{written} vs {now}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_huge_file_is_read_only_up_to_the_limit() {
    let dir = scratch("huge");
    let mut text = String::from(UNLOCKED);
    text.push_str(&"x".repeat(MAX_LOG_BYTES as usize * 2));
    put(&dir, "RUN0001.TXT", &text);
    let report = latest_in_dir(&dir).unwrap().unwrap();
    assert_eq!(report.status, BootStatus::Unlocked, "the facts are at the top of a log");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_log_that_is_not_text_still_yields_a_report() {
    let dir = scratch("binary");
    std::fs::write(dir.join("RUN0001.TXT"), [0xFF, 0xFE, 0x00, 0x80]).unwrap();
    let report = latest_in_dir(&dir).unwrap().unwrap();
    assert_eq!((report.run.as_str(), report.status), ("RUN0001", BootStatus::NotUnlocked));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn facts_deep_in_a_normal_sized_log_are_read() {
    // The real logs are about 16 KiB and keep their result near the end; the limit is far above that.
    let dir = scratch("deep");
    let mut text = String::from("RTX 4060 Laptop unlock build=auto profile=x\n");
    text.push_str(&"asset line with some detail\n".repeat(4_000));
    text.push_str("POST-GOP SS0=0x88888888 SS1=0x8\nTRANSACTION OK: done\n");
    assert!(text.len() > 100_000 && (text.len() as u64) < MAX_LOG_BYTES);
    put(&dir, "RUN0001.TXT", &text);
    assert_eq!(latest_in_dir(&dir).unwrap().unwrap().status, BootStatus::Unlocked);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_path_that_is_not_a_directory_is_an_error_not_no_log() {
    let dir = scratch("notdir");
    let file = dir.join("RUN0001.TXT");
    std::fs::write(&file, UNLOCKED).unwrap();
    let err = latest_in_dir(&file).unwrap_err();
    assert!(err.contains("could not list"), "{err}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_limit_is_a_quarter_of_a_mebibyte() {
    assert_eq!(MAX_LOG_BYTES, 262_144);
}
