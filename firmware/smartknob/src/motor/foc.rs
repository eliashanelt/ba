use crate::pid;

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

pub struct Foc<const PWM_RESOLUTION: u16> {
    velocity_pid: pid::PIDController,
    motion_control_type: MotionControlType,
    torque_control_type: TorqueControlType,
    shaft_velocity: f32,
}

impl<const PWM_RESOLUTION: u16> Foc<PWM_RESOLUTION> {
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
