//! Fixed-point PI and PID controllers.

/// A fixed-point PI controller.
pub struct PIController {
    k_p: f32,
    integral: IntegralComponent,
}

impl PIController {
    /// Create a new PI controller with the given gains.
    pub fn new(k_p: f32, k_i: f32) -> Self {
        Self {
            k_p,
            integral: IntegralComponent { k_i, integral: 0.0 },
        }
    }

    /// Update the PI controller, returning the new output value.
    pub fn update(&mut self, measurement: f32, setpoint: f32, dt: f32) -> f32 {
        let error = measurement - setpoint;
        self.k_p * error + self.integral.update(error, dt)
    }
}

/// A fixed-point PID controller.
///
/// Uses the derivative-on-measurement technique to avoid derivative kicks on
/// setpoint changes.
pub struct PIDController {
    k_p: f32,
    integral: IntegralComponent,
    derivative: DerivativeComponent,
    output_ramp: Option<f32>,
    limit: Option<f32>,
    prev_output: Option<f32>,
}

impl PIDController {
    /// Create a new PID controller with the given gains.
    pub fn new(k_p: f32, k_i: f32, k_d: f32, output_ramp: Option<f32>, limit: Option<f32>) -> Self {
        Self {
            k_p,
            integral: IntegralComponent { k_i, integral: 0.0 },
            derivative: DerivativeComponent {
                k_d,
                last_measurement: None,
            },
            output_ramp,
            limit,
            prev_output: None,
        }
    }

    /// Update the PID controller, returning the new output value.
    pub fn update(&mut self, measurement: f32, setpoint: f32, dt: f32) -> f32 {
        let error = measurement - setpoint;
        let mut output = self.k_p * error
            + self.integral.update(error, dt)
            + self.derivative.update(measurement, dt);
        if let Some(limit) = self.limit {
            output = output.clamp(limit, limit);
        }
        let prev_output = self.prev_output.unwrap_or(0.0);
        if let Some(output_ramp) = self.output_ramp {
            let output_rate = (prev_output - output) / dt;
            if output_rate > output_ramp {
                output = prev_output + output_ramp * dt;
            } else if output_rate < -output_ramp {
                output = prev_output - output_ramp * dt;
            }
        }
        self.prev_output = Some(output);
        output
    }
}

struct IntegralComponent {
    k_i: f32,
    integral: f32,
}

impl IntegralComponent {
    fn update(&mut self, error: f32, dt: f32) -> f32 {
        self.integral += self.k_i * error * dt;
        self.integral
    }
}

struct DerivativeComponent {
    k_d: f32,
    last_measurement: Option<f32>,
}

impl DerivativeComponent {
    fn update(&mut self, measurement: f32, dt: f32) -> f32 {
        let derivative = self
            .last_measurement
            .map(|last| (measurement - last) / dt)
            .unwrap_or(0.0);

        self.last_measurement = Some(measurement);

        self.k_d * derivative
    }
}

fn constrain(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}
