use defmt::error;

use crate::{
    bldc::BldcMotor,
    pid::PIDController,
    util::{normalize_angle, Direction, LowPassFilter},
};

pub mod trap_maps;

const DEF_VEL_LIM: f32 = 20.0;
const DEF_POWER_SUPPLY: f32 = 12.0;
const DEF_CURRENT_LIM: f32 = 2.0;
const DEF_INDEX_SEARCH_TARGET_VELOCITY: f32 = 1.0;
const DEF_VOLTAGE_SENSOR_ALIGN: f32 = 3.0;
const DEF_MOTION_DOWNSAMPLE: u32 = 0;
const DEF_MON_DOWNSAMPLE: u32 = 10;
const MIN_ANGLE_DETECT_MOVEMENT: f32 = 0.05;
const NOT_SET: f32 = -12345.0;
const HIGH_IMPEDANCE: i32 = 0;

// Monitoring bitmap constants
const MON_TARGET: u8 = 0b1000000;
const MON_VOLT_Q: u8 = 0b0100000;
const MON_VOLT_D: u8 = 0b0010000;
const MON_CURR_Q: u8 = 0b0001000;
const MON_CURR_D: u8 = 0b0000100;
const MON_VEL: u8 = 0b0000010;
const MON_ANGLE: u8 = 0b0000001;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MotorStatus {
    MotorUninitialized,
    MotorInitializing,
    MotorUncalibrated,
    MotorCalibrating,
    MotorReady,
    MotorError,
    MotorCalibFailed,
    MotorInitFailed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModulationType {
    SinePWM,
    SpaceVectorPWM,
    Trapezoid120,
    Trapezoid150,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MotionControlType {
    Torque,
    Velocity,
    Angle,
    VelocityOpenloop,
    AngleOpenloop,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TorqueControlType {
    Voltage,
    DcCurrent,
    FocCurrent,
}

pub trait AngleSensor {
    fn electrical_angle(&self) -> f32;
    fn mechanical_angle(&self) -> f32;
    fn velocity(&self) -> f32;
}

pub trait Motor {
    fn move_to(new_target: f32);
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct DQ {
    pub d: f32,
    pub q: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Foc<S: AngleSensor> {
    pub target: f32,
    pub feed_forward_velocity: f32,
    pub shaft_angle: f32,
    pub electrical_angle: f32,
    pub shaft_velocity: f32,
    pub current_sp: f32,
    pub shaft_velocity_sp: f32,
    pub shaft_angle_sp: f32,
    pub voltage: DQ,
    pub current: DQ,
    pub voltage_bemf: f32,
    pub u_alpha: f32,
    pub u_beta: f32,

    // Motor configuration parameters
    pub voltage_sensor_align: f32,
    pub velocity_index_search: f32,

    // Motor physical parameters
    pub phase_resistance: Option<f32>,
    pub pole_pairs: i32,
    pub kv_rating: Option<f32>,
    pub phase_inductance: Option<f32>,

    // Limiting variables
    pub voltage_limit: f32,
    pub current_limit: f32,
    pub velocity_limit: f32,

    pub enabled: bool,
    pub motor_status: MotorStatus,

    // PWM modulation related variables
    pub modulation: ModulationType,
    pub modulation_centered: bool,

    // Configuration structures
    pub torque_controller: TorqueControlType,
    pub controller: MotionControlType,

    // Controllers and filters (placeholders)
    pub pid_current_q: PIDController,
    pub pid_current_d: PIDController,
    pub lpf_current_q: LowPassFilter,
    pub lpf_current_d: LowPassFilter,
    pub pid_velocity: PIDController,
    pub p_angle: PIDController,
    pub lpf_velocity: LowPassFilter,
    pub lpf_angle: LowPassFilter,

    pub motion_downsample: u32,
    pub motion_cnt: u32,

    // Sensor related variables
    pub sensor_offset: f32,
    pub zero_electric_angle: f32,
    pub sensor_direction: Option<Direction>,
    pub pp_check_result: bool,

    // Monitoring variables
    pub monitor_downsample: u32,
    pub monitor_start_char: Option<char>,
    pub monitor_end_char: Option<char>,
    pub monitor_separator: char,
    pub monitor_decimals: usize,
    pub monitor_variables: u8,
    monitor_cnt: u32,

    // References to external components
    pub angle_sensor: Option<S>,
    //pub current_sense: Option<Box<dyn CurrentSense>>,
}

impl<S: AngleSensor> Default for Foc<S> {
    fn default() -> Self {
        Self {
            target: 0.0,
            feed_forward_velocity: 0.0,
            shaft_angle: 0.0,
            electrical_angle: 0.0,
            shaft_velocity: 0.0,
            current_sp: 0.0,
            shaft_velocity_sp: 0.0,
            shaft_angle_sp: 0.0,
            voltage: DQ::default(),
            current: DQ::default(),
            voltage_bemf: 0.0,
            u_alpha: 0.0,
            u_beta: 0.0,

            voltage_sensor_align: DEF_VOLTAGE_SENSOR_ALIGN,
            velocity_index_search: DEF_INDEX_SEARCH_TARGET_VELOCITY,

            phase_resistance: None,
            pole_pairs: 0,
            kv_rating: None,
            phase_inductance: None,

            voltage_limit: DEF_POWER_SUPPLY,
            current_limit: DEF_CURRENT_LIM,
            velocity_limit: DEF_VEL_LIM,

            enabled: false,
            motor_status: MotorStatus::MotorUninitialized,

            modulation: ModulationType::SinePWM,
            modulation_centered: true,

            torque_controller: TorqueControlType::Voltage,
            controller: MotionControlType::Torque,

            pid_current_q: PIDController::new(0.0, 0.0, 0.0, None, Some(DEF_POWER_SUPPLY)),
            pid_current_d: PIDController::new(0.0, 0.0, 0.0, None, Some(DEF_POWER_SUPPLY)),
            lpf_current_q: LowPassFilter::new(0.005),
            lpf_current_d: LowPassFilter::new(0.005),
            pid_velocity: PIDController::new(0.0, 0.0, 0.0, None, None),
            p_angle: PIDController::new(0.0, 0.0, 0.0, None, Some(DEF_VEL_LIM)),
            lpf_velocity: LowPassFilter::new(0.005),
            lpf_angle: LowPassFilter::new(0.0),

            motion_downsample: DEF_MOTION_DOWNSAMPLE,
            motion_cnt: 0,

            sensor_offset: 0.0,
            zero_electric_angle: NOT_SET,
            sensor_direction: None,
            pp_check_result: false,

            monitor_downsample: DEF_MON_DOWNSAMPLE,
            monitor_start_char: None,
            monitor_end_char: None,
            monitor_separator: '\t',
            monitor_decimals: 4,
            monitor_variables: MON_TARGET | MON_VOLT_Q | MON_VEL | MON_ANGLE,
            monitor_cnt: 0,

            angle_sensor: None,
        }
    }
}

impl<S: AngleSensor> Foc<S> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn shaft_angle(&mut self) -> f32 {
        if let Some(sensor) = &mut self.angle_sensor {
            let angle = self.lpf_angle.update(sensor.electrical_angle());
            match self.sensor_direction {
                Some(Direction::CW) | None => angle - self.sensor_offset,
                Some(Direction::CCW) => -angle - self.sensor_offset,
            }
        } else {
            self.shaft_angle
        }
    }

    pub fn shaft_velocity(&mut self) -> f32 {
        if let Some(sensor) = &mut self.angle_sensor {
            let velocity = self.lpf_velocity.update(sensor.velocity());
            velocity
        } else {
            self.shaft_velocity
        }
    }

    pub fn electrical_angle(&mut self) -> f32 {
        if let Some(sensor) = &mut self.angle_sensor {
            let dir_multiplier = if sensor.velocity() > 0.0 { 1.0 } else { -1.0 };
            normalize_angle(
                dir_multiplier * (self.pole_pairs as f32) * sensor.mechanical_angle()
                    - self.zero_electric_angle,
            )
        } else {
            self.electrical_angle
        }
    }
}
