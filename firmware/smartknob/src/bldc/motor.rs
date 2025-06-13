use core::f32::consts::{PI, TAU};

use defmt::error;
use embassy_time::Instant;
use libm::{cosf, sinf};

use crate::{
    foc::{
        trap_maps::{HIGH_IMPEDANCE, TRAP_120_MAP, TRAP_150_MAP},
        AngleSensor, Foc, ModulationType, MotionControlType, TorqueControlType,
    },
    util::{normalize_angle, SQRT3_2},
};

use super::{driver::PhaseState, BldcDriver};

pub struct BldcMotor<S: AngleSensor> {
    pub foc: Foc<S>,
    pub driver: BldcDriver,
    pub u_a: f32,
    pub u_b: f32,
    pub u_c: f32,
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
            u_a: 0.0,
            u_b: 0.0,
            u_c: 0.0,
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
        /*if let Some(current_sense) = &mut self.foc.current_sense {
            current_sense.disable();
        }*/

        self.driver.disable();

        self.foc.enabled = false;
    }

    pub fn enable(&mut self) {
        self.driver.enable();

        // Reset PIDs
        self.foc.pid_velocity.reset();
        self.foc.p_angle.reset();
        self.foc.pid_current_q.reset();
        self.foc.pid_current_d.reset();

        self.foc.enabled = true;
    }

    pub fn update(&mut self) {
        if self.foc.controller == MotionControlType::AngleOpenloop
            || self.foc.controller == MotionControlType::VelocityOpenloop
            || !self.foc.enabled
        {
            return;
        }
        self.foc.electrical_angle = self.foc.electrical_angle();

        match self.foc.torque_controller {
            TorqueControlType::Voltage => {}
            TorqueControlType::DcCurrent => {
                error!("Control Type DC Current, but no current sense specified");
                return;
            }
            TorqueControlType::FocCurrent => {
                error!("Control Type foc Current, but no current sense specified");
                return;
            }
        }
        self.set_phase_voltage(
            self.foc.voltage.q,
            self.foc.voltage.d,
            self.foc.electrical_angle,
        );
    }

    pub fn set_phase_voltage(&mut self, u_q: f32, u_d: f32, electrical_angle: f32) {
        match self.foc.modulation {
            ModulationType::Trapezoid120 => {
                let sector = (6.0 * (normalize_angle(electrical_angle + PI / 6.0) / TAU)) as usize;
                let center = if self.foc.modulation_centered {
                    self.driver.voltage_limit / 2.0
                } else {
                    u_q
                };

                let map = TRAP_120_MAP[sector];
                if map[0] == HIGH_IMPEDANCE {
                    self.u_a = center;
                    self.u_b = map[1] as f32 * u_q + center;
                    self.u_c = map[2] as f32 * u_q + center;
                    self.driver.phase_states = [PhaseState::Off, PhaseState::On, PhaseState::On];
                } else if map[1] == HIGH_IMPEDANCE {
                    self.u_a = map[0] as f32 * u_q + center;
                    self.u_b = center;
                    self.u_c = map[2] as f32 * u_q + center;
                    self.driver.phase_states = [PhaseState::On, PhaseState::Off, PhaseState::On];
                } else {
                    self.u_a = map[0] as f32 * u_q + center;
                    self.u_b = map[1] as f32 * u_q + center;
                    self.u_c = center;
                    self.driver.phase_states = [PhaseState::On, PhaseState::On, PhaseState::Off];
                }
            }

            ModulationType::Trapezoid150 => {
                let sector = (12.0 * (normalize_angle(electrical_angle + PI / 6.0) / TAU)) as usize;
                let center = if self.foc.modulation_centered {
                    self.driver.voltage_limit / 2.0
                } else {
                    u_q
                };

                let map = TRAP_150_MAP[sector];
                if map[0] == HIGH_IMPEDANCE {
                    self.u_a = center;
                    self.u_b = map[1] as f32 * u_q + center;
                    self.u_c = map[2] as f32 * u_q + center;
                    self.driver.phase_states = [PhaseState::Off, PhaseState::On, PhaseState::On];
                } else if map[1] == HIGH_IMPEDANCE {
                    self.u_a = map[0] as f32 * u_q + center;
                    self.u_b = center;
                    self.u_c = map[2] as f32 * u_q + center;
                    self.driver.phase_states = [PhaseState::On, PhaseState::Off, PhaseState::On];
                } else if map[2] == HIGH_IMPEDANCE {
                    self.u_a = map[0] as f32 * u_q + center;
                    self.u_b = map[1] as f32 * u_q + center;
                    self.u_c = center;
                    self.driver.phase_states = [PhaseState::On, PhaseState::On, PhaseState::Off];
                } else {
                    self.u_a = map[0] as f32 * u_q + center;
                    self.u_b = map[1] as f32 * u_q + center;
                    self.u_c = map[2] as f32 * u_q + center;
                    self.driver.phase_states.fill(PhaseState::On);
                }
            }

            ModulationType::SinePWM | ModulationType::SpaceVectorPWM => {
                // inverse Park + Clarke transformation
                let sa = sinf(electrical_angle);
                let ca = cosf(electrical_angle);

                let u_alpha = ca * u_d - sa * u_q;
                let u_beta = sa * u_d + ca * u_q;

                self.u_a = u_alpha;
                self.u_b = -0.5 * u_alpha + SQRT3_2 * u_beta;
                self.u_c = -0.5 * u_alpha - SQRT3_2 * u_beta;

                let mut center = self.driver.voltage_limit / 2.0;

                if self.foc.modulation == ModulationType::SpaceVectorPWM {
                    let u_min = self.u_a.min(self.u_b.min(self.u_c));
                    let u_max = self.u_a.max(self.u_b.max(self.u_c));
                    center -= (u_max + u_min) / 2.0;
                }

                if !self.foc.modulation_centered {
                    let u_min = self.u_a.min(self.u_b.min(self.u_c));
                    self.u_a -= u_min;
                    self.u_b -= u_min;
                    self.u_c -= u_min;
                } else {
                    self.u_a += center;
                    self.u_b += center;
                    self.u_c += center;
                }
            }
        }
    }
}
