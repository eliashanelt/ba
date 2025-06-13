use core::f32::consts::PI;

use crate::util::Direction;

#[derive(Debug, Clone, Copy)]
pub struct SmartKnobConfig {
    /// Set the integer position.
    pub position: i32,

    /// Set the fractional position. Typical range: (-snap_point, snap_point).
    pub sub_position_unit: f32,

    /// Position is normally only applied when it changes—but you can force
    /// re‐application via a nonce bump. Must be < 256.
    pub position_nonce: u8,

    /// Minimum position allowed.
    pub min_position: i32,

    /// Maximum position allowed.
    /// If equal to `min_position` → only one allowed position.
    /// If < `min_position` → bounds are disabled.
    pub max_position: i32,

    /// Angular “width” of each position/detent, in radians.
    pub position_width_radians: f32,

    /// Strength of detents (0 = off, >1 = unstable).
    pub detent_strength_unit: f32,

    /// Strength of endstop torque at the bounds (0 = off).
    pub endstop_strength_unit: f32,

    /// Fractional threshold where the position will step.
    /// Typical range: (0.5, 1.5).
    pub snap_point: f32,

    /// Arbitrary NUL-terminated C string (up to 64 chars + NUL).
    pub id: [u8; 65],

    /// How many entries in `detent_positions` are valid.
    pub detent_positions_count: usize,

    /// Up to 5 “magnetic” detent positions.
    pub detent_positions: [i32; 5],

    /// Bias for asymmetric detents; typical = 0.
    pub snap_point_bias: f32,

    /// Hue (0–255) for 8 ring LEDs, if supported.
    pub led_hue: i16,
}
impl SmartKnobConfig {
    pub fn check(&self) -> Result<(), ConfigError> {
        if self.detent_strength_unit < 0.0 {
            return Err(ConfigError::DetentStrengthNegative);
        }
        if self.endstop_strength_unit < 0.0 {
            return Err(ConfigError::EndstopStrengthNegative);
        }
        if self.snap_point < 0.5 {
            return Err(ConfigError::SnapPointTooSmall);
        }
        if self.detent_positions_count > self.detent_positions.len() {
            return Err(ConfigError::DetentPositionsCountTooLarge);
        }
        if self.snap_point_bias < 0.0 {
            return Err(ConfigError::SnapPointBiasNegative);
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub enum ConfigError {
    DetentStrengthNegative,
    EndstopStrengthNegative,
    SnapPointTooSmall,
    DetentPositionsCountTooLarge,
    SnapPointBiasNegative,
}

impl core::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ConfigError::DetentStrengthNegative => {
                write!(f, "detent_strength_unit cannot be negative")
            }
            ConfigError::EndstopStrengthNegative => {
                write!(f, "endstop_strength_unit cannot be negative")
            }
            ConfigError::SnapPointTooSmall => {
                write!(f, "snap_point must be >= 0.5 for stability")
            }
            ConfigError::DetentPositionsCountTooLarge => {
                write!(f, "detent_positions_count is too large")
            }
            ConfigError::SnapPointBiasNegative => {
                write!(
                    f,
                    "snap_point_bias cannot be negative or there is risk of instability"
                )
            }
        }
    }
}

impl Default for SmartKnobConfig {
    fn default() -> Self {
        Self {
            position_width_radians: 60.0 * PI / 180.0,
            endstop_strength_unit: 0.0,
            snap_point: 0.5,
            detent_positions_count: 0,
            detent_positions: [0; 5],
            snap_point_bias: 0.0,
            detent_strength_unit: 0.0,
            position: 0,
            sub_position_unit: 0.0,
            position_nonce: 0,
            min_position: 0,
            max_position: 0,
            id: [0; 65],
            led_hue: 0,
        }
    }
}

pub struct PersistentConfiguration {
    pub zero_electrical_offset: f32,
    pub direction_cw: Direction,
    pub pole_pairs: u32,
}
