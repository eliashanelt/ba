use core::f32::consts::PI;

use driver::BldcDriver;
use foc::Foc;

use crate::{config::SmartKnobConfig, util::Direction};

mod driver;
mod foc;
pub mod mt6701;

pub const FOC_PID_P: f32 = 4.0;
pub const FOC_PID_I: f32 = 0.0;
pub const FOC_PID_D: f32 = 0.04;
pub const FOC_PID_OUTPUT_RAMP: f32 = 10000.0;
pub const FOC_PID_LIMIT: f32 = 10.0;

pub const DEAD_ZONE_DETENT_PERCENT: f32 = 0.2;
pub const DEAD_ZONE_RAD: f32 = 1.0 * PI / 180.0;

pub const IDLE_VELOCITY_EWMA_ALPHA: f32 = 0.001;
pub const IDLE_VELOCITY_RAD_PER_SEC: f32 = 0.05;
pub const IDLE_CORRECTION_DELAY_MILLIS: u32 = 500;
pub const IDLE_CORRECTION_MAX_ANGLE_RAD: f32 = 5.0 * PI / 180.0;
pub const IDLE_CORRECTION_RATE_ALPHA: f32 = 0.0005;
pub const FOC_VOLTAGE_LIMIT: f32 = 5.0;

// trap_maps.rs

/// Phase state values: 1 = positive, -1 = negative, 0 = high-impedance
pub const HIGH_IMPEDANCE: i8 = 0;

/// Each entry represents a 60° step for the three-phase trap waveform
pub const TRAP_120_MAP: [[i8; 3]; 6] = [
    [HIGH_IMPEDANCE, 1, -1],
    [-1, 1, HIGH_IMPEDANCE],
    [-1, HIGH_IMPEDANCE, 1],
    [HIGH_IMPEDANCE, -1, 1],
    [1, -1, HIGH_IMPEDANCE],
    [1, HIGH_IMPEDANCE, -1],
];

/// Each entry represents a 30° step for the three-phase trap waveform
pub const TRAP_150_MAP: [[i8; 3]; 12] = [
    [HIGH_IMPEDANCE, 1, -1],
    [-1, 1, -1],
    [-1, 1, HIGH_IMPEDANCE],
    [-1, 1, 1],
    [-1, HIGH_IMPEDANCE, 1],
    [-1, -1, 1],
    [HIGH_IMPEDANCE, -1, 1],
    [1, -1, 1],
    [1, -1, HIGH_IMPEDANCE],
    [1, -1, -1],
    [1, HIGH_IMPEDANCE, -1],
    [1, 1, -1],
];

pub enum Command {
    Calibrate,
    Config(SmartKnobConfig),
    Haptic(Press),
}

pub enum Press {
    Short,
    Long,
}

pub struct BldcMotor {
    pub driver: BldcDriver,
    pub foc: Foc,
    pub pole_pairs: u32,
    pub zero_electric_angle: f32,
    pub sensor_direction: Direction,
    pub voltage_limit: f32,
}

impl BldcMotor {
    pub fn init_foc(&mut self) {
        //self.driver.set_phase_duties(0.5, 0.5, 0.5);
    }
    pub fn drive_elec(&mut self, angle_elec: f32) {
        use libm::sinf;
        const MOD: f32 = 0.25;
        let m = MOD.min(self.voltage_limit / FOC_VOLTAGE_LIMIT);

        let (va, vb, vc) = (
            sinf(angle_elec) * m + 0.5,
            sinf(angle_elec - 2.0 * PI / 3.0) * m + 0.5,
            sinf(angle_elec + 2.0 * PI / 3.0) * m + 0.5,
        );
        self.driver.set_phase_duties(va, vb, vc);
    }

    /* ------------------------------------------------------------------ */
    /* 2.  `move_to` (ABSOLUTE) – wrapper expected by calibration         */
    /* ------------------------------------------------------------------ */
    pub fn move_to(&mut self, new_target: f32) {
        if self.foc.motion_cnt + 1 < self.foc.motion_downsample {
            return;
        }

        motion_cnt = 0;

        //self.drive_elec(angle_elec);
        //match self.control_type {}
    }

    pub fn calibrate(&mut self) {}

    /*pub fn move_by(&mut self, delta_elec: f32) {
        // keep track of the current commanded angle in a field
        self.target += delta_elec;
        self.drive_elec(self.target);
    }*/
}
