fn as_compare_value<const MAX: u16>(value: TwoPhaseReferenceFrame) -> [u16; 3] {
    modulate(value).map(|v| {
        // scale into [0 .. MAX+1] range
        let scaled = (v + 1.0) * ((MAX as f32) + 1.0) / 2.0;
        // add 0.5 for rounding-to-nearest, then clamp into [0 .. MAX]
        let rounded = (scaled + 0.5).clamp(0.0, MAX as f32);
        // truncate to u16
        rounded as u16
    })
}

fn modulate(value: TwoPhaseReferenceFrame) -> [f32; 3] {
    // Convert alpha/beta to x/y/z
    let sqrt_3_alpha = SQRT_3 * value.alpha;
    let beta = value.beta;
    let x = beta;
    let y = (beta + sqrt_3_alpha) / 2.0;
    let z = (beta - sqrt_3_alpha) / 2.0;

    // Calculate which sector the value falls in
    let sector: u8 = match (
        x.is_sign_positive(),
        y.is_sign_positive(),
        z.is_sign_positive(),
    ) {
        (true, true, false) => 1,
        (_, true, true) => 2,
        (true, false, true) => 3,
        (false, false, true) => 4,
        (_, false, false) => 5,
        (false, true, false) => 6,
    };

    // Map a,b,c values to three phase
    let (ta, tb, tc);
    match sector {
        1 | 4 => {
            ta = x - z;
            tb = x + z;
            tc = -x + z;
        }
        2 | 5 => {
            ta = y - z;
            tb = y + z;
            tc = -y - z;
        }
        3 | 6 => {
            ta = y - x;
            tb = -y + x;
            tc = -y - x;
        }
        _ => unreachable!("invalid sector"),
    }

    [ta, tb, tc]
}

#[derive(Debug, Clone)]
pub struct ThreePhaseBalancedReferenceFrame {
    pub a: f32,
    pub b: f32,
}

#[derive(Debug, Clone)]
pub struct TwoPhaseReferenceFrame {
    pub alpha: f32,
    pub beta: f32,
}

#[derive(Debug, Clone)]
pub struct RotatingReferenceFrame {
    pub d: f32,
    pub q: f32,
}

pub fn inverse_park(
    cos_angle: f32,
    sin_angle: f32,
    inputs: RotatingReferenceFrame,
) -> TwoPhaseReferenceFrame {
    TwoPhaseReferenceFrame {
        // Eq10
        alpha: cos_angle * inputs.d - sin_angle * inputs.q,
        // Eq11
        beta: sin_angle * inputs.d + cos_angle * inputs.q,
    }
}

pub fn inverse_clarke(inputs: TwoPhaseReferenceFrame) -> ThreePhaseReferenceFrame {
    ThreePhaseReferenceFrame {
        // Eq5
        a: inputs.alpha,
        // Eq6
        b: (-inputs.alpha + SQRT_3 * inputs.beta) / 2.0,
        // Eq7
        c: (-inputs.alpha - SQRT_3 * inputs.beta) / 2.0,
    }
}
#[derive(Debug, Clone)]
pub struct ThreePhaseReferenceFrame {
    pub a: f32,
    pub b: f32,
    pub c: f32,
}
pub fn clarke(inputs: ThreePhaseBalancedReferenceFrame) -> TwoPhaseReferenceFrame {
    TwoPhaseReferenceFrame {
        // Eq3
        alpha: inputs.a,
        // Eq4
        beta: FRAC_1_SQRT_3 * (inputs.a + 2.0 * inputs.b),
    }
}

pub fn park(
    cos_angle: f32,
    sin_angle: f32,
    inputs: TwoPhaseReferenceFrame,
) -> RotatingReferenceFrame {
    RotatingReferenceFrame {
        // Eq8
        d: cos_angle * inputs.alpha + sin_angle * inputs.beta,
        // Eq9
        q: cos_angle * inputs.beta - sin_angle * inputs.alpha,
    }
}

pub struct OldFoc<const PWM_RESOLUTION: u16> {
    flux_current_controller: pid::PIController,
    torque_current_controller: pid::PIController,
    velocity_pid: pid::PIDController,
    motion_control_type: MotionControlType,
    torque_control_type: TorqueControlType,
    shaft_velocity: f32,
}

impl<const PWM_RESOLUTION: u16> OldFoc<PWM_RESOLUTION> {
    pub fn new(
        flux_current_controller: pid::PIController,
        torque_current_controller: pid::PIController,
    ) -> Self {
        Self {
            flux_current_controller,
            torque_current_controller,
            velocity_pid: pid::PIDController::new(1.0, 2.0, 3.0, None, None),
            motion_control_type: MotionControlType::Torque,
            torque_control_type: TorqueControlType::Voltage,
            shaft_velocity: 0.0,
        }
    }

    pub fn update(
        &mut self,
        currents: [f32; 2],
        angle: f32,
        desired_torque: f32,
        dt: f32,
    ) -> [u16; 3] {
        let sin_angle = sinf(angle);
        let cos_angle = cosf(angle);

        let orthogonal_current = clarke(ThreePhaseBalancedReferenceFrame {
            a: currents[0],
            b: currents[1],
        });

        let rotating_current = park(cos_angle, sin_angle, orthogonal_current);

        let v_d = self
            .flux_current_controller
            .update(rotating_current.d, 0.0, dt);
        let v_q = self
            .torque_current_controller
            .update(rotating_current.q, desired_torque, dt);

        // Inverse Park transform
        let orthogonal_voltage = inverse_park(
            cos_angle,
            sin_angle,
            RotatingReferenceFrame { d: v_d, q: v_q },
        );
        as_compare_value::<PWM_RESOLUTION>(orthogonal_voltage)
    }
}
