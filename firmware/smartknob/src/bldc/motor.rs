use core::f32::consts::{PI, TAU};

use defmt::error;
use embassy_time::Instant;
use libm::{cosf, sinf};

use crate::{
    foc::{
        trap_maps::{HIGH_IMPEDANCE, TRAP_120_MAP, TRAP_150_MAP},
        AngleSensor, Foc, ModulationType, MotionControlType, TorqueControlType,
    },
    util::{normalize_angle, RPM_TO_RADS, SQRT3, SQRT3_2},
};

use super::{driver::PhaseState, BldcDriver};

pub struct BldcMotor {
    pub foc: Foc,
    pub driver: BldcDriver,
    pub u_a: f32,
    pub u_b: f32,
    pub u_c: f32,
    open_loop_timestamp: Option<Instant>,
}

impl BldcMotor {
    pub fn new(
        pole_pairs: i32,
        phase_resistance: Option<f32>,
        kv_rating: Option<f32>,
        phase_inductance: Option<f32>,
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

    pub fn move_to(&mut self, new_target: f32) {
        if self.foc.controller != MotionControlType::AngleOpenloop
            && self.foc.controller != MotionControlType::VelocityOpenloop
        {
            self.foc.shaft_angle = self.foc.shaft_angle();
        }
        self.foc.shaft_velocity = self.foc.shaft_velocity();

        if !self.foc.enabled {
            return;
        }
        if new_target.is_finite() {
            self.foc.target = new_target
        }

        if let Some(kv) = self.foc.kv_rating {
            self.foc.voltage_bemf = self.foc.shaft_velocity / (kv * SQRT3) / RPM_TO_RADS;
        }

        if true {
            //if self.current_sense.is_none() {
            if let Some(r) = self.foc.phase_resistance {
                self.foc.current.q = (self.foc.voltage.q - self.foc.voltage_bemf) / r;
            }
        }

        match self.foc.controller {
            MotionControlType::Torque => {
                if self.foc.torque_controller == TorqueControlType::Voltage {
                    if let Some(r) = self.foc.phase_resistance {
                        self.foc.voltage.q = self.foc.target * r + self.foc.voltage_bemf;
                    } else {
                        self.foc.voltage.q = self.foc.target;
                    }

                    self.foc.voltage.q = self
                        .foc
                        .voltage
                        .q
                        .clamp(-self.foc.voltage_limit, self.foc.voltage_limit);

                    self.foc.voltage.d = if let Some(l) = self.foc.phase_inductance {
                        (-self.foc.target
                            * self.foc.shaft_velocity
                            * self.foc.pole_pairs as f32
                            * l)
                            .clamp(-self.foc.voltage_limit, self.foc.voltage_limit)
                    } else {
                        0.0
                    };
                } else {
                    self.foc.current_sp = self.foc.target;
                }
            }
            MotionControlType::Velocity => {
                self.foc.shaft_velocity_sp = self.foc.target;
                self.foc.current_sp = self
                    .foc
                    .pid_velocity
                    .update(self.foc.shaft_velocity_sp - self.foc.shaft_velocity);

                if self.foc.torque_controller == TorqueControlType::Voltage {
                    if let Some(r) = self.foc.phase_resistance {
                        self.foc.voltage.q = self.foc.current_sp * r
                            + self
                                .foc
                                .voltage_bemf
                                .clamp(-self.driver.voltage_limit, self.driver.voltage_limit);
                    } else {
                        self.foc.voltage.q = self.foc.current_sp;
                    }

                    self.foc.voltage.d = if let Some(l) = self.foc.phase_inductance {
                        (-self.foc.current_sp
                            * self.foc.shaft_velocity
                            * self.foc.pole_pairs as f32
                            * l)
                            .clamp(-self.driver.voltage_limit, self.driver.voltage_limit)
                    } else {
                        0.0
                    };
                }
            }
            MotionControlType::Angle => {
                self.foc.shaft_angle_sp = self.foc.target;
                self.foc.shaft_velocity_sp = self.foc.feed_forward_velocity
                    + self
                        .foc
                        .p_angle
                        .update(self.foc.shaft_angle_sp - self.foc.shaft_angle);
                self.foc.shaft_velocity_sp = self
                    .foc
                    .shaft_velocity_sp
                    .clamp(-self.foc.velocity_limit, self.foc.velocity_limit);
                self.foc.current_sp = self
                    .foc
                    .pid_velocity
                    .update(self.foc.shaft_velocity_sp - self.foc.shaft_velocity);

                if self.foc.torque_controller == TorqueControlType::Voltage {
                    if let Some(r) = self.foc.phase_resistance {
                        self.foc.voltage.q = (self.foc.current_sp * r + self.foc.voltage_bemf)
                            .clamp(-self.foc.voltage_limit, self.foc.voltage_limit);
                    } else {
                        self.foc.voltage.q = self.foc.current_sp;
                    }

                    self.foc.voltage.d = if let Some(l) = self.foc.phase_inductance {
                        (-self.foc.current_sp
                            * self.foc.shaft_velocity
                            * self.foc.pole_pairs as f32
                            * l)
                            .clamp(-self.foc.voltage_limit, self.driver.voltage_limit)
                    } else {
                        0.0
                    };
                }
            }
            MotionControlType::VelocityOpenloop => {
                self.foc.shaft_velocity_sp = self.foc.target;
                self.foc.voltage.q = self.velocity_openloop(self.foc.shaft_velocity_sp);
                self.foc.voltage.d = 0.0;
            }
            MotionControlType::AngleOpenloop => {
                self.foc.shaft_angle_sp = self.foc.target;
                self.foc.voltage.q = self.angle_openloop(self.foc.shaft_angle_sp);
                self.foc.voltage.d = 0.0;
            }
        }
    }

    pub fn update_foc(&mut self) {
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
        self.driver.set_phase_duties(self.u_a, self.u_b, self.u_c);
    }

    pub fn velocity_openloop(&mut self, target_velocity: f32) -> f32 {
        let now = Instant::now();

        // Calculate sample time from last call
        let ts = if let Some(last_timestamp) = self.open_loop_timestamp {
            let duration = now.duration_since(last_timestamp);
            let ts_seconds = 1_000_000 as f32 / duration.as_micros() as f32;

            // Quick fix for strange cases (timer overflow + timestamp not defined)
            if ts_seconds <= 0.0 || ts_seconds > 0.5 {
                1e-3 // 1ms default
            } else {
                ts_seconds
            }
        } else {
            1e-3 // 1ms default for first call
        };

        // Calculate the necessary angle to achieve target velocity
        self.foc.shaft_angle = normalize_angle(self.foc.shaft_angle + target_velocity * ts);
        // For display purposes
        self.foc.shaft_velocity = target_velocity;

        // Use voltage limit or current limit
        let mut u_q = self.foc.voltage_limit;

        if let Some(phase_resistance) = self.foc.phase_resistance {
            u_q = (self.foc.current_limit * phase_resistance + self.foc.voltage_bemf.abs())
                .clamp(-self.foc.voltage_limit, self.foc.voltage_limit);

            // Recalculate the current
            self.foc.current.q = (u_q - self.foc.voltage_bemf.abs()) / phase_resistance;
        }

        // Set the maximal allowed voltage with the necessary angle
        let electrical_angle = self.foc.shaft_angle * self.foc.pole_pairs as f32;
        self.set_phase_voltage(u_q, 0.0, electrical_angle);

        // Save timestamp for next call
        self.open_loop_timestamp = Some(now);

        u_q
    }

    /// Open loop angle control - generates movement towards target angle with velocity limit
    pub fn angle_openloop(&mut self, target_angle: f32) -> f32 {
        let now = Instant::now();

        // Calculate sample time from last call
        let ts = if let Some(last_timestamp) = self.open_loop_timestamp {
            let duration = now.duration_since(last_timestamp);
            let ts_seconds = 1_000_000 as f32 / duration.as_micros() as f32;

            // Quick fix for strange cases (timer overflow + timestamp not defined)
            if ts_seconds <= 0.0 || ts_seconds > 0.5 {
                1e-3 // 1ms default
            } else {
                ts_seconds
            }
        } else {
            1e-3 // 1ms default for first call
        };

        // Calculate the necessary angle to move from current position towards target angle
        // with maximal velocity (velocity_limit)
        let angle_diff = target_angle - self.foc.shaft_angle;
        let max_angle_step = self.foc.velocity_limit.abs() * ts;

        if angle_diff.abs() > max_angle_step {
            let direction = if angle_diff > 0.0 { 1.0 } else { -1.0 };
            self.foc.shaft_angle += direction * max_angle_step;
            self.foc.shaft_velocity = self.foc.velocity_limit * direction;
        } else {
            self.foc.shaft_angle = target_angle;
            self.foc.shaft_velocity = 0.0;
        }

        // Use voltage limit or current limit
        let mut u_q = self.foc.voltage_limit;

        if let Some(phase_resistance) = self.foc.phase_resistance {
            u_q = (self.foc.current_limit * phase_resistance + self.foc.voltage_bemf.abs())
                .clamp(-self.foc.voltage_limit, self.foc.voltage_limit);

            // Recalculate the current
            self.foc.current.q = (u_q - self.foc.voltage_bemf.abs()) / phase_resistance;
        }

        // Set the maximal allowed voltage with the necessary angle
        // Normalize the angle before calculating electrical angle for precision
        let normalized_shaft_angle = normalize_angle(self.foc.shaft_angle);
        let electrical_angle = normalized_shaft_angle * self.foc.pole_pairs as f32;
        self.set_phase_voltage(u_q, 0.0, electrical_angle);

        // Save timestamp for next call
        self.open_loop_timestamp = Some(now);

        u_q
    }
}
