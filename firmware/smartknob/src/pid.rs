use embassy_time::Instant;

/// PID controller struct
#[derive(Clone, Copy, Debug)]
pub struct PIDController {
    /// Proportional gain
    pub p: f32,
    /// Integral gain
    pub i: f32,
    /// Derivative gain
    pub d: f32,
    /// Maximum speed of change of the output value [units/second]
    pub output_ramp: Option<f32>,
    /// Maximum output value
    pub limit: Option<f32>,

    // Internal state
    error_prev: f32,
    output_prev: f32,
    integral_prev: f32,
    timestamp_prev: Option<Instant>,
}

impl PIDController {
    /// Create a new PID controller
    ///
    /// # Arguments
    /// * `p` - Proportional gain
    /// * `i` - Integral gain  
    /// * `d` - Derivative gain
    /// * `output_ramp` - Maximum speed of change of the output value [units/second]
    /// * `limit` - Maximum output value
    pub fn new(p: f32, i: f32, d: f32, output_ramp: Option<f32>, limit: Option<f32>) -> Self {
        Self {
            p,
            i,
            d,
            output_ramp,
            limit,
            error_prev: 0.0,
            output_prev: 0.0,
            integral_prev: 0.0,
            timestamp_prev: None,
        }
    }

    /// Calculate PID output for given error
    ///
    /// # Arguments
    /// * `error` - The tracking error (setpoint - measured_value)
    ///
    /// # Returns
    /// The PID controller output
    pub fn update(&mut self, error: f32) -> f32 {
        let now = Instant::now();

        // Calculate sample time from last call
        let ts = if let Some(last_timestamp) = self.timestamp_prev {
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

        // u(s) = (P + I/s + Ds)e(s)
        // Discrete implementations

        // Proportional part: u_p = P * e(k)
        let proportional = self.p * error;

        // Tustin transform of the integral part: u_ik = u_ik_1 + I*Ts/2*(ek + ek_1)
        let mut integral = self.integral_prev + self.i * ts * 0.5 * (error + self.error_prev);
        // Antiwindup - limit the integral
        if let Some(limit) = self.limit {
            integral = integral.clamp(-limit, limit);
        }

        // Discrete derivation: u_dk = D(ek - ek_1)/Ts
        let derivative = self.d * (error - self.error_prev) / ts;

        // Sum all the components
        let mut output = proportional + integral + derivative;
        // Antiwindup - limit the output variable
        if let Some(limit) = self.limit {
            output = output.clamp(-limit, limit);
        }

        // If output ramp is defined
        if let Some(output_ramp) = self.output_ramp {
            // Limit the acceleration by ramping the output
            let output_rate = (output - self.output_prev) / ts;
            if output_rate > output_ramp {
                output = self.output_prev + output_ramp * ts;
            } else if output_rate < -output_ramp {
                output = self.output_prev - output_ramp * ts;
            }
        }

        // Save values for next iteration
        self.integral_prev = integral;
        self.output_prev = output;
        self.error_prev = error;
        self.timestamp_prev = Some(now);

        output
    }

    /// Reset the PID controller internal state
    pub fn reset(&mut self) {
        self.integral_prev = 0.0;
        self.output_prev = 0.0;
        self.error_prev = 0.0;
        self.timestamp_prev = None;
    }

    /// Set new PID gains
    pub fn set_gains(&mut self, p: f32, i: f32, d: f32) {
        self.p = p;
        self.i = i;
        self.d = d;
    }

    /// Set output limits
    pub fn set_limits(&mut self, limit: Option<f32>, output_ramp: Option<f32>) {
        self.limit = limit;
        self.output_ramp = output_ramp;
    }
}

// Helper function to constrain values (equivalent to C++ _constrain)
fn constrain(value: f32, min_val: f32, max_val: f32) -> f32 {
    value.clamp(min_val, max_val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_pid_controller_creation() {
        let pid = PIDController::new(1.0, 0.1, 0.01, 10.0, 5.0);
        assert_eq!(pid.p, 1.0);
        assert_eq!(pid.i, 0.1);
        assert_eq!(pid.d, 0.01);
        assert_eq!(pid.output_ramp, 10.0);
        assert_eq!(pid.limit, 5.0);
    }

    #[test]
    fn test_pid_proportional_only() {
        let mut pid = PIDController::new(2.0, 0.0, 0.0, 0.0, 10.0);
        let error = 1.0;
        let output = pid.calculate(error);
        assert_eq!(output, 2.0); // P * error = 2.0 * 1.0
    }

    #[test]
    fn test_pid_output_limiting() {
        let mut pid = PIDController::new(10.0, 0.0, 0.0, 0.0, 5.0);
        let error = 1.0;
        let output = pid.calculate(error);
        assert_eq!(output, 5.0); // Should be limited to 5.0
    }

    #[test]
    fn test_pid_reset() {
        let mut pid = PIDController::new(1.0, 1.0, 1.0, 0.0, 10.0);

        // Run the controller to build up internal state
        pid.calculate(1.0);
        thread::sleep(Duration::from_millis(10));
        pid.calculate(1.0);

        // Reset and verify
        pid.reset();
        assert_eq!(pid.integral_prev, 0.0);
        assert_eq!(pid.output_prev, 0.0);
        assert_eq!(pid.error_prev, 0.0);
        assert!(pid.timestamp_prev.is_none());
    }
}
