//! @module esp_live_test
//! @description Live check of the EFI system partition reader. Ignored by default: it needs an elevated
//! session (the helper's LocalSystem account can read the partition; an ordinary user cannot) and a machine
//! the UEFI tensor unlocker has run on. Build with `cargo test --test unit --no-run`, then run the test
//! executable elevated with `esp_live --ignored --nocapture`.
use nitrotray::service::esp::{find_log_dir, EspUnlockLog};
use nitrotray::service::unlock_log::UnlockLogSource;

#[test]
#[ignore = "needs an elevated session and a machine the unlocker has run on"]
fn the_partition_is_found_without_mounting_it_and_its_newest_log_is_read() {
    let dir = find_log_dir().expect("volume enumeration").expect("a FAT volume holding the unlocker's log directory");
    println!("log directory: {}", dir.display());
    let report = EspUnlockLog.latest().expect("read").expect("at least one RUN log");
    println!("{report:?}");
    assert!(report.run.to_ascii_uppercase().starts_with("RUN"));
    assert!(report.written_unix.is_some());
}
