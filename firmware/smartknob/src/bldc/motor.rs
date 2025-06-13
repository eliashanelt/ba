use embassy_time::Instant;

use crate::foc::{AngleSensor, Foc, TorqueControlType};

use super::BldcDriver;

pub struct BldcMotor<S: AngleSensor> {
    pub foc: Foc<S>,
    pub driver: BldcDriver,
    pub ua: f32,
    pub ub: f32,
    pub uc: f32,
    open_loop_timestamp: Option<Instant>,
}

impl<S: AngleSensor> BldcMotor<S> {
    pub fn new(
        pole_pairs: i32,
        phase_resistance: f32,
        kv_rating: Option<f32>,
        phase_inductance: f32,
        driver: BldcDriver,
    ) -> Self {
        let mut motor = Self {
            foc: Foc::new(),
            driver,
            ua: 0.0,
            ub: 0.0,
            uc: 0.0,
            open_loop_timestamp: None,
        };

        motor.foc.pole_pairs = pole_pairs;
        motor.foc.phase_resistance = phase_resistance;
        motor.foc.kv_rating = kv_rating;
        motor.foc.phase_inductance = phase_inductance;
        motor.foc.torque_controller = TorqueControlType::Voltage;

        motor
    }

    pub fn disable(&mut self) {
        if let Some(current_sense) = &mut self.foc.current_sense {
            current_sense.disable();
        }

        if let Some(driver) = &mut self.driver {
            driver.set_pwm(0.0, 0.0, 0.0);
            driver.disable();
        }

        self.foc.enabled = false;
    }

    pub fn enable(&mut self) {
        driver.enable();
        driver.set_pwm(0.0, 0.0, 0.0);

        // Reset PIDs
        self.foc.pid_velocity.reset();
        self.foc.p_angle.reset();
        self.foc.pid_current_q.reset();
        self.foc.pid_current_d.reset();

        self.foc.enabled = true;
    }
}
