use core::f32::consts::PI;

use driver::BldcDriver;
use foc::Foc;

use crate::{config::SmartKnobConfig, util::Direction};

mod driver;
mod foc;
pub mod mt6701;

const FOC_PID_P: f32 = 4.0;
const FOC_PID_I: f32 = 0.0;
const FOC_PID_D: f32 = 0.04;
const FOC_PID_OUTPUT_RAMP: f32 = 10000.0;
const FOC_PID_LIMIT: f32 = 10.0;

const DEAD_ZONE_DETENT_PERCENT: f32 = 0.2;
const DEAD_ZONE_RAD: f32 = 1.0 * PI / 180.0;

const IDLE_VELOCITY_EWMA_ALPHA: f32 = 0.001;
const IDLE_VELOCITY_RAD_PER_SEC: f32 = 0.05;
const IDLE_CORRECTION_DELAY_MILLIS: u32 = 500;
const IDLE_CORRECTION_MAX_ANGLE_RAD: f32 = 5.0 * PI / 180.0;
const IDLE_CORRECTION_RATE_ALPHA: f32 = 0.0005;
const FOC_VOLTAGE_LIMIT: f32 = 5.0;

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
    driver: BldcDriver,
    foc: Foc<4095>,
    pole_pairs: u32,
    zero_electric_angle: f32,
    sensor_direction: Direction,
    voltage_limit: f32,
    shaft_angle: f32,
    target: f32,
}

impl BldcMotor {
    pub fn init_foc(&mut self) {
        self.driver.set_phase_duties(0.5, 0.5, 0.5);
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
    pub fn move_to(&mut self, target: f32) {
        //self.drive_elec(angle_elec);
        //match self.control_type {}
    }

    /* ------------------------------------------------------------------ */
    /* 3.  `move_by` (RELATIVE) – optional helper for UI / haptics        */
    /* ------------------------------------------------------------------ */
    pub fn move_by(&mut self, delta_elec: f32) {
        // keep track of the current commanded angle in a field
        self.target += delta_elec;
        self.drive_elec(self.target);
    }
}
