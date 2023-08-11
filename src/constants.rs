use std::time::Duration;

pub const SUBMERGE_TIMEOUT: Duration = Duration::from_secs(15);
pub const SUBMERGE_HEAVE: f32 = 1.9;
pub const SUBMERGE_HEAVE_TOLERANCE: f32 = 0.1;

pub const ALIGN_GATE_TIMEOUT: Duration = Duration::from_secs(10);
pub const ALIGN_GATE_YAW: f32 = 100.;
pub const ALIGN_GATE_YAW_TOLERANCE: f32 = 2.;

pub const APPROACH_GATE_TIMEOUT: Duration = Duration::from_secs(25);
pub const APPROACH_GATE_SURGE_SPEED: f32 = 0.2;
