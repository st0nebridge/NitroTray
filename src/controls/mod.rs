//! @module controls
//! @description Control-domain types: fan modes, performance modes and custom fan curves.
//!
//! @dependencies controls::fan_mode, controls::performance_mode, controls::fan_curve
pub mod fan_curve;
pub mod fan_mode;
pub mod performance_mode;

pub use fan_curve::{CurveError, CurvePoint, FanCurve, FanCurveProfile};
pub use fan_mode::FanMode;
pub use performance_mode::PerformanceMode;
