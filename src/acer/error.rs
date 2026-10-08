//! @module acer::error
//! @description Error vocabulary for Acer firmware access, shared by the backend contract and discovery.
//!
//! @output `AcerError` with user-readable `Display` text.
//! @dependencies none

/// Why a hardware call failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcerError {
    /// WMI refused the caller (not elevated).
    AccessDenied,
    /// The AcerGamingFunction interface is absent on this machine.
    NotPresent,
    /// The firmware returned a non-zero status byte.
    Firmware(u8),
    /// The capability was not discovered or is model-gated off.
    Unsupported(&'static str),
    InvalidArgument(String),
    /// COM/WMI transport failure.
    Transport(String),
}

impl std::fmt::Display for AcerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessDenied => write!(f, "access to the Acer firmware interface was denied"),
            Self::NotPresent => write!(f, "the Acer gaming firmware interface is not present"),
            Self::Firmware(s) => write!(f, "the firmware rejected the request (status 0x{s:02X})"),
            Self::Unsupported(what) => write!(f, "{what} is not supported on this model"),
            Self::InvalidArgument(m) => write!(f, "invalid argument: {m}"),
            Self::Transport(m) => write!(f, "firmware interface error: {m}"),
        }
    }
}
