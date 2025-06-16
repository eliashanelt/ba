use core::f32::consts::PI;

use defmt::info;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::Delay;
use embedded_hal::delay::DelayNs;
use libm::fabsf;

use crate::{
    bldc::BldcMotor,
    config::{PersistentConfiguration, SmartKnobConfig},
    pid,
    util::{millis, Direction},
};

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

pub enum Command {
    Calibrate,
    Config(SmartKnobConfig),
    Haptic(Press),
}

pub enum Press {
    Short,
    Long,
}

static MOTOR_CH: Channel<CriticalSectionRawMutex, Command, 1> = Channel::new();

#[embassy_executor::task]
pub async fn motor_task(mut motor: BldcMotor) {
    let persistant_config = PersistentConfiguration {
        zero_electrical_offset: 0.0,
        direction_cw: Direction::CW,
        pole_pairs: 8,
    };

    //motor.init();
    motor.foc.zero_electric_angle = persistant_config.zero_electrical_offset;
    motor.init_foc();
    motor.foc.monitor_downsample = 0;
    let mut current_detent_center = motor.foc.shaft_angle;
    let mut config = SmartKnobConfig::default();
    let mut current_position = 0;

    let mut latest_sub_position_unit = 0.0;

    let mut idle_check_velocity_ewma = 0.0;
    let mut last_idle_start: u32 = 0;
    let mut last_publish: u32 = 0;
    loop {
        motor.update_foc();
        if let Ok(command) = MOTOR_CH.try_receive() {
            match command {
                Command::Calibrate => {
                    motor.foc.pid_velocity = pid::PIDController::new(
                        FOC_PID_P,
                        FOC_PID_I,
                        FOC_PID_D,
                        Some(FOC_PID_OUTPUT_RAMP),
                        Some(FOC_PID_LIMIT),
                    );

                    motor.calibrate().await;
                    motor.init_foc().await;
                }
                Command::Config(new_config) => {
                    if let Err(e) = new_config.check() {
                        panic!("Invalid smartknob config: {e}");
                    }
                    let mut position_updated = false;

                    if new_config.position != config.position
                        || new_config.sub_position_unit != config.sub_position_unit
                        || new_config.position_nonce != config.position_nonce
                    {
                        info!("Applying position change");
                        current_position = new_config.position;
                        position_updated = true;
                    }
                    //TODO!
                    if new_config.min_position <= new_config.max_position {
                        // Only check bounds if min/max indicate bounds are active (min >= max)
                        if current_position < new_config.min_position {
                            current_position = new_config.min_position;
                            info!("Adjusting position to min");
                        } else if current_position > new_config.max_position {
                            current_position = new_config.max_position;
                            info!("Adjusting position to max");
                        }
                    }

                    if position_updated
                        || new_config.position_width_radians != config.position_width_radians
                    {
                        info!("Adjusting detent center");
                        let new_sub_position = if position_updated {
                            new_config.sub_position_unit
                        } else {
                            latest_sub_position_unit
                        };
                        let shaft_angle = -motor.foc.shaft_angle;
                        current_detent_center =
                            shaft_angle + new_sub_position * new_config.position_width_radians;
                    }
                    config = new_config;
                    info!("Got new config");

                    let derivative_lower_strength = config.detent_strength_unit * 0.08;
                    let derivative_upper_strength = config.detent_strength_unit * 0.02;
                    let derivative_position_width_lower = 3.0_f32.to_radians();
                    let derivative_position_width_upper = 8.0_f32.to_radians();

                    let raw = derivative_lower_strength
                        + (derivative_upper_strength - derivative_lower_strength)
                            / (derivative_position_width_upper - derivative_position_width_lower)
                            * (config.position_width_radians - derivative_position_width_lower);

                    motor.foc.pid_velocity.d = if config.detent_positions_count > 0 {
                        0.0
                    } else {
                        let min = derivative_lower_strength.min(derivative_upper_strength);
                        let max = derivative_lower_strength.max(derivative_upper_strength);
                        raw.clamp(min, max)
                    };
                }
                //DONE
                Command::Haptic(press) => {
                    let (strength, foc_ticks) = match press {
                        Press::Short => (5.0, 3),
                        Press::Long => (20.0, 6),
                    };
                    motor.move_to(strength);
                    for _ in 0..foc_ticks {
                        motor.update_foc();
                        Delay.delay_ms(1);
                    }
                    motor.move_to(-strength);
                    for _ in 0..foc_ticks {
                        motor.update_foc();
                        Delay.delay_ms(1);
                    }
                    motor.move_to(0.0);
                    motor.update_foc();
                }
            }

            idle_check_velocity_ewma = motor.foc.shaft_velocity * IDLE_VELOCITY_EWMA_ALPHA
                + idle_check_velocity_ewma * (1.0 - IDLE_VELOCITY_EWMA_ALPHA);

            if fabsf(idle_check_velocity_ewma) > IDLE_VELOCITY_RAD_PER_SEC {
                last_idle_start = 0;
            } else if last_idle_start == 0 {
                last_idle_start = millis();
            }
            if last_idle_start > 0
                && millis() - last_idle_start > IDLE_CORRECTION_DELAY_MILLIS
                && fabsf(motor.foc.shaft_angle - current_detent_center)
                    < IDLE_CORRECTION_MAX_ANGLE_RAD
            {
                current_detent_center = motor.foc.shaft_angle * IDLE_CORRECTION_RATE_ALPHA
                    + current_detent_center * (1.0 - IDLE_CORRECTION_RATE_ALPHA);
            }

            let mut angle_to_detent_center = -motor.foc.shaft_angle - current_detent_center;

            let snap_point_radians = config.position_width_radians * config.snap_point;
            let bias_radians = config.position_width_radians * config.snap_point_bias;
            let snap_point_radians_decrease = snap_point_radians
                + (if current_position <= 0 {
                    bias_radians
                } else {
                    -bias_radians
                });
            let snap_point_radians_increase = -snap_point_radians
                + (if current_position >= 0 {
                    -bias_radians
                } else {
                    bias_radians
                });

            // number of discrete positionsld
            let num_positions: i32 = config.max_position - config.min_position + 1;

            // move detent center and position if we’ve crossed snap thresholds
            if angle_to_detent_center > snap_point_radians_decrease
                && (num_positions <= 0 || current_position > config.min_position)
            {
                current_detent_center += config.position_width_radians;
                angle_to_detent_center -= config.position_width_radians;
                current_position -= 1;
            } else if angle_to_detent_center < snap_point_radians_increase
                && (num_positions <= 0 || current_position < config.max_position)
            {
                current_detent_center -= config.position_width_radians;
                angle_to_detent_center += config.position_width_radians;
                current_position += 1;
            }

            // compute the fractional (sub-position) unit
            latest_sub_position_unit = -angle_to_detent_center / config.position_width_radians;

            // clamp into a small “dead zone” around center
            let dead_zone_min =
                (-config.position_width_radians * DEAD_ZONE_DETENT_PERCENT).max(-DEAD_ZONE_RAD);
            let dead_zone_max =
                (config.position_width_radians * DEAD_ZONE_DETENT_PERCENT).min(DEAD_ZONE_RAD);
            let dead_zone_adjustment = angle_to_detent_center.clamp(dead_zone_min, dead_zone_max);

            // check if we’re pushing past the hard bounds
            let out_of_bounds = num_positions > 0
                && ((angle_to_detent_center > 0.0 && current_position == config.min_position)
                    || (angle_to_detent_center < 0.0 && current_position == config.max_position));

            // apply to your motor controller
            motor.foc.pid_velocity.limit = Some(10.0); // or: if out_of_bounds { 10 } else { 3 };
            motor.foc.pid_velocity.p = if out_of_bounds {
                config.endstop_strength_unit * 4.0
            } else {
                config.detent_strength_unit * 4.0
            };
            if fabsf(motor.foc.shaft_velocity) > 60.0 {
                motor.move_to(0.0);
            } else {
                //let input = -angle
            }
            let torque = 0.0; //motor.foc.pid_velocity()
            motor.move_to(torque);
        }
    }
}
