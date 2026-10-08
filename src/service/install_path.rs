//! @module service::install_path
//! @description Where the LocalSystem helper binary is allowed to live: an administrators-only folder
//! under Program Files — never next to the per-user tray (a user-writable path would let any user
//! process swap the binary and run code as SYSTEM).
//!
//! @input  The Program Files directory (from the environment, or given by tests).
//! @output Target directory/exe paths; a check that a path sits inside the protected location.
//! @dependencies meta
use std::path::{Path, PathBuf};

pub const SERVICE_EXE: &str = "NitroTrayService.exe";

/// 64-bit Program Files (`ProgramW6432` wins over a 32-bit view of `ProgramFiles`).
pub fn program_files() -> Option<PathBuf> {
    ["ProgramW6432", "ProgramFiles"].iter().find_map(std::env::var_os).map(PathBuf::from)
}

pub fn service_dir(program_files: &Path) -> PathBuf {
    program_files.join(crate::meta::APP_NAME)
}

pub fn service_exe(program_files: &Path) -> PathBuf {
    service_dir(program_files).join(SERVICE_EXE)
}

/// Case-insensitive, separator-tolerant "is `path` inside `program_files`".
pub fn is_protected_location(path: &Path, program_files: &Path) -> bool {
    let norm = |p: &Path| p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_ascii_lowercase();
    let (p, root) = (norm(path), norm(program_files));
    !root.is_empty() && p.starts_with(&format!("{root}\\"))
}
