use core::f32::consts::{PI, TAU};

use defmt::{debug, error, info};
use embassy_time::{Delay, Instant};
use embedded_hal::delay::DelayNs;
use libm::{atan2f, cosf, fabsf, sinf};

use crate::{
    foc::{
        trap_maps::{HIGH_IMPEDANCE, TRAP_120_MAP, TRAP_150_MAP},
        AngleSensor, Foc, ModulationType, MotionControlType, MotorStatus, TorqueControlType,
    },
    tasks::motor::FOC_VOLTAGE_LIMIT,
    util::{normalize_angle, Direction, RPM_TO_RADS, SQRT3, SQRT3_2},
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

    pub async fn move_to(&mut self, new_target: f32) {
        if self.foc.controller != MotionControlType::AngleOpenloop
            && self.foc.controller != MotionControlType::VelocityOpenloop
        {
            self.foc.shaft_angle = self.foc.shaft_angle().await;
        }
        self.foc.shaft_velocity = self.foc.shaft_velocity().await;

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

    pub async fn update_foc(&mut self) {
        if self.foc.controller == MotionControlType::AngleOpenloop
            || self.foc.controller == MotionControlType::VelocityOpenloop
            || !self.foc.enabled
        {
            return;
        }
        self.foc.electrical_angle = self.foc.electrical_angle().await;

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

    pub async fn absolute_zero_search(&mut self) -> Result<(), ()> {
        info!("MOT: Index search...");

        let Some(angle_sensor) = self.foc.angle_sensor.as_mut() else {
            error!("No angle sensor available for absolute zero search");
            return Err(());
        };

        // Store current limits
        let limit_vel = self.foc.velocity_limit;
        let limit_volt = self.foc.voltage_limit;

        // Set search parameters
        self.foc.velocity_limit = self.foc.velocity_index_search;
        self.foc.voltage_limit = self.foc.voltage_sensor_align;
        self.foc.shaft_angle = 0.0;

        // Search for the absolute zero with small velocity
        while angle_sensor.needs_zero_search().await && self.foc.shaft_angle < TAU {
            self.angle_openloop(1.5 * TAU);

            // Call update - important for some sensors not to lose count
            // Not needed for the search itself
            //angle_sensor.update().await;
        }

        // Disable motor
        self.set_phase_voltage(0.0, 0.0, 0.0);

        // Reinit the limits
        self.foc.velocity_limit = limit_vel;
        self.foc.voltage_limit = limit_volt;

        // Check if the zero was found
        if angle_sensor.needs_zero_search().await {
            error!("MOT: Error: Not found!");
            Err(())
        } else {
            error!("MOT: Success!");
            Ok(())
        }
    }

    pub async fn init_foc(&mut self) -> Result<(), ()> {
        debug!("MOT: Initializing FOC...");

        // Set motor status to calibrating
        self.foc.motor_status = MotorStatus::MotorCalibrating;

        // If angle sensor is available, initialize it
        if let Some(angle_sensor) = self.foc.angle_sensor.as_mut() {
            // Align sensor first
            let align_result = self.align_sensor().await;
            if align_result.is_err() {
                error!("MOT: Sensor alignment failed during FOC initialization");
                self.foc.motor_status = MotorStatus::MotorError;
                return Err(());
            }

            // Update sensor and set initial shaft angle
            angle_sensor.update().await;
            self.foc.shaft_angle = self.foc.shaft_angle().await;

            info!("MOT: Initial shaft angle: {}", self.foc.shaft_angle);
        } else {
            info!("MOT: No angle sensor configured - running in open loop mode only");
        }

        // Reset all controllers to known state
        self.foc.pid_velocity.reset();
        self.foc.p_angle.reset();
        self.foc.pid_current_q.reset();
        self.foc.pid_current_d.reset();

        // Initialize voltage and current values
        self.foc.voltage.q = 0.0;
        self.foc.voltage.d = 0.0;
        self.foc.current.q = 0.0;
        self.foc.current.d = 0.0;

        // Set initial targets
        self.foc.target = 0.0;
        self.foc.shaft_velocity_sp = 0.0;
        self.foc.shaft_angle_sp = self.foc.shaft_angle;
        self.foc.current_sp = 0.0;

        // Reset timing
        self.open_loop_timestamp = None;

        // Set motor status to ready
        self.foc.motor_status = MotorStatus::MotorReady;

        info!("MOT: FOC initialization complete");
        Ok(())
    }
    async fn calibrate(&mut self) -> Result<(), ()> {
        let Some(angle_sensor) = self.foc.angle_sensor.as_mut() else {
            error!("error during calibration, no angle sensor configured");
            return Err(());
        };

        self.foc.controller = MotionControlType::AngleOpenloop;
        self.foc.pole_pairs = 1;
        self.foc.zero_electric_angle = 0.0;
        self.foc.sensor_direction = Some(Direction::CW);
        self.init_foc();

        let mut alpha = 0.0;

        self.foc.voltage_limit = FOC_VOLTAGE_LIMIT;
        self.move_to(alpha);

        for _ in 0..200 {
            self.move_to(alpha);
            Delay.delay_ms(1);
        }
        let start_angle = angle_sensor.read_angle().await;

        while alpha < 3.0 * 2.0 * PI {
            self.move_to(alpha);
            Delay.delay_ms(1);
            alpha += 0.01;
        }

        let end_angle = angle_sensor.electrical_angle().await;

        self.foc.voltage_limit = 0.0;
        self.move_to(alpha);

        let moved_angle = (end_angle - start_angle).abs();

        if moved_angle < 30.0_f32.to_radians() || moved_angle > 180.0_f32.to_radians() {
            error!(
                "Unexpected sensor change: start={}, end={}, moved={}",
                start_angle, end_angle, moved_angle
            );
            return Err(());
        }

        info!(
            "sensor change: start={}, end={}, moved={}",
            start_angle, end_angle, moved_angle
        );

        let direction = if end_angle > start_angle {
            Direction::CW
        } else {
            Direction::CCW
        };
        info!(
            "sensor measures positive change for positive motor rotaion: {}",
            direction == Direction::CW
        );
        self.foc.zero_electric_angle = 0.0;
        self.foc.sensor_direction = Some(direction);
        self.init_foc();

        let elec_revs = 20;
        info!("Going to measure {} electrical revolutions", elec_revs);
        self.foc.voltage_limit = FOC_VOLTAGE_LIMIT;
        self.move_to(alpha);
        info!("Going to electrical zero");
        let dest = alpha + 2.0 * PI;
        while alpha < dest {
            self.move_to(alpha);
            alpha += 0.03;
        }
        info!("Pause");
        Delay.delay_ms(1000);
        info!("Measuring");
        let start_angle = f32::from(self.foc.sensor_direction) * mt6701.read_angle().await;
        let dest = alpha + elec_revs as f32 * TAU;
        while alpha < dest {
            self.move_to(alpha);
            alpha += 0.03;
        }
        info!("Pause");
        for _ in 0..1000 {
            self.move_by(alpha);
            Delay.delay_ms(1);
        }
        let end_angle = f32::from(motor.sensor_direction) * mt6701.read_angle().await;
        self.foc.voltage_limit = 0.0;
        self.move_to(alpha);

        if fabsf(self.foc.shaft_angle - self.foc.target) > PI / 180.0 {
            error!("motor did not reach target");
            return Err(());
        }
        info!("moved: {}", end_angle - start_angle);
        let elec_per_mech: f32 = elec_revs as f32 * TAU / (end_angle - start_angle);
        info!(
            "electrical angle / mechanical angle (i.e. pole pairs) = {}",
            elec_per_mech
        );
        if elec_per_mech < 3.0 || elec_per_mech > 12.0 {
            error!("Unexpected calculated pole pairs: {}", elec_per_mech);
            return Err(());
        }

        let measured_pole_pairs: u32 = (elec_per_mech as f32 + 0.5) as u32;
        info!("Pole pairs set to {}", measured_pole_pairs);

        Delay.delay_ms(1000);

        motor.voltage_limit = FOC_VOLTAGE_LIMIT;
        motor.move_to(alpha);
        let mut offset_x = 0.0;
        let mut offset_y = 0.0;
        let dest1 = (floorf(alpha / 2.0 * PI) + measured_pole_pairs as f32 / 2.0) * 2.0 * PI;
        let dest2 = floorf(alpha / 2.0 * PI) * 2.0 * PI;

        while alpha < dest1 {
            motor.move_to(alpha);
            Delay.delay_ms(200);
            let real_elec_angle = normalize_angle(alpha);
            let measured_elec_angle = normalize_angle(
                f32::from(motor.sensor_direction)
                    * measured_pole_pairs as f32
                    * mt6701.read_angle().await,
            );

            let offset_angle = measured_elec_angle - real_elec_angle;
            offset_x += cosf(offset_angle);
            offset_y += sinf(offset_angle);
            info!(
                "{},{},{}",
                (real_elec_angle).to_degrees(),
                (measured_elec_angle).to_degrees(),
                (normalize_angle(offset_angle)).to_degrees()
            );
            alpha += 0.4;
        }
        while alpha > dest2 {
            self.move_to(alpha);
            Delay.delay_ms(200);
            let real_elec_angle = normalize_angle(alpha);
            let measured_elec_angle = normalize_angle(
                f32::from(self.sensor_direction)
                    * measured_pole_pairs as f32
                    * mt6701.read_angle().await,
            );

            let offset_angle = measured_elec_angle - real_elec_angle;
            offset_x += cosf(offset_angle);
            offset_y += sinf(offset_angle);
            info!(
                "{},{},{}",
                (real_elec_angle).to_degrees().to_degrees(),
                (measured_elec_angle).to_degrees().to_degrees(),
                (normalize_angle(offset_angle)).to_degrees()
            );
            alpha -= 0.4;
        }
        self.foc.voltage_limit = 0.0;
        self.move_to(alpha);

        let avg_offset_angle = atan2f(offset_y, offset_x);

        self.foc.pole_pairs = measured_pole_pairs as i32;
        self.foc.zero_electric_angle = avg_offset_angle + 3.0 * PI / 2.0;
        self.foc.voltage_limit = FOC_VOLTAGE_LIMIT;
        self.foc.controller = MotionControlType::Torque;

        info!(
            "calibration results: 0elec_angle={}, direction is cw={}",
            self.foc.zero_electric_angle,
            self.foc.sensor_direction == Some(Direction::CW)
        );

        Ok(())
    }
}
