use crate::{
    pid::{self, PIDController},
    util::{Direction, LowPassFilter},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionControlType {
    Torque,
    Velocity,
    Angle,
    VelocityOpenLoop,
    AngleOpenLoop,
}

#[derive(Clone, Copy)]
pub enum TorqueControlType {
    Voltage,
    DcCurrent,
    FocCurrent,
}

struct Dq {
    d: f32,
    q: f32,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModulationType {
    /// Sinusoidal PWM modulation
    SinePwm,
    /// Space Vector PWM modulation
    SpaceVectorPwm,
    /// 120° trapezoidal (six-step) modulation
    Trapezoid120,
    /// 150° trapezoidal modulation
    Trapezoid150,
}

/// Operational states of the FOC motor controller
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotorStatus {
    /// Motor is not yet initialized
    Uninitialized,
    /// Motor initialization in progress
    Initializing,
    /// Initialized but not calibrated (open-loop only)
    Uncalibrated,
    /// Calibration in progress
    Calibrating,
    /// Initialized and calibrated (closed-loop possible)
    Ready,
    /// Recoverable error (e.g. overcurrent protection active)
    Error,
    /// Calibration failed (possibly recoverable)
    CalibrationFailed,
    /// Initialization failed (not recoverable)
    InitializationFailed,
}
pub trait Sensor {
    /// Mechanical angle [rad]
    fn get_mechanical_angle(&mut self) -> f32;
    /// Raw velocity [rad/s]
    fn get_velocity(&mut self) -> f32;
}

pub struct FocMotor<S: Sensor> {
    // Targets and setpoints
    pub target: f32,
    pub feed_forward_velocity: f32,
    pub shaft_angle: f32,
    pub electrical_angle: f32,
    pub shaft_velocity: f32,
    pub current_sp: f32,
    pub shaft_velocity_sp: f32,
    pub shaft_angle_sp: f32,

    // Voltages & currents
    pub voltage: Dq,
    pub current: Dq,
    pub voltage_bemf: f32,

    // Clarke/Park intermediate
    pub u_alpha: f32,
    pub u_beta: f32,

    // Sensor alignment
    pub sensor_offset: f32,
    pub zero_electric_angle: Option<f32>,
    pub sensor_direction: Direction,

    // Configuration
    pub phase_resistance: f32,
    pub pole_pairs: u32,
    pub kv_rating: f32,
    pub phase_inductance: f32,
    pub voltage_sensor_align: f32,
    pub voltage_limit: f32,
    pub current_limit: f32,
    pub velocity_limit: f32,

    // Control & modulation
    pub motion_control_type: MotionControlType,
    pub torque_control_type: TorqueControlType,
    pub foc_modulation: ModulationType,

    // Status & enable
    pub enabled: bool,
    pub motor_status: MotorStatus,

    // Sensors
    sensor: Option<S>,

    // Controllers & filters
    pub pid_current_q: PIDController,
    pub pid_current_d: PIDController,
    pub pid_velocity: PIDController,
    pub p_angle: PIDController,
    pub lpf_current_q: LowPassFilter,
    pub lpf_current_d: LowPassFilter,
    pub lpf_velocity: LowPassFilter,
    pub lpf_angle: LowPassFilter,

    // Monitoring
    pub monitor_downsample: u32,
    monitor_cnt: u32,
}
impl Foc {
    pub fn init(&mut self) {}

    pub fn update(&mut self) {
        //open loop
        if self.motion_control_type == MotionControlType::AngleOpenLoop
            || self.motion_control_type == MotionControlType::VelocityOpenLoop
        {
            return;
        }

        //set_phase_voltage()
    }
}
