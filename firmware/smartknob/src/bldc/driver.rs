use embedded_hal::pwm::SetDutyCycle;
use esp_hal::{mcpwm::operator::PwmPin, peripherals::MCPWM0, time::Rate};

const PWM_FREQUENCY: Rate = Rate::from_khz(40);

#[derive(Clone, Copy)]
pub enum PhaseState {
    Off,
    On,
    High,
    Low,
}

pub struct BldcDriver {
    pub uh: PwmPin<'static, MCPWM0, 0, true>,
    pub ul: PwmPin<'static, MCPWM0, 0, false>,
    pub vh: PwmPin<'static, MCPWM0, 1, true>,
    pub vl: PwmPin<'static, MCPWM0, 1, false>,
    pub wh: PwmPin<'static, MCPWM0, 2, true>,
    pub wl: PwmPin<'static, MCPWM0, 2, false>,
    pub dead_zone: f32,
    pub pwm_frequency: Rate,
    pub phase_states: [PhaseState; 3],
}

impl BldcDriver {
    pub fn init(
        uh: PwmPin<'static, MCPWM0, 0, true>,
        ul: PwmPin<'static, MCPWM0, 0, false>,
        vh: PwmPin<'static, MCPWM0, 1, true>,
        vl: PwmPin<'static, MCPWM0, 1, false>,
        wh: PwmPin<'static, MCPWM0, 2, true>,
        wl: PwmPin<'static, MCPWM0, 2, false>,
    ) -> Self {
        Self {
            uh,
            ul,
            vh,
            vl,
            wh,
            wl,
            dead_zone: 0.02,
            pwm_frequency: PWM_FREQUENCY,
            phase_states: [PhaseState::Off; 3],
        }
    }

    pub fn enable(&mut self) {
        self.phase_states = [PhaseState::On; 3];
        self.set_phase_duties(0.0, 0.0, 0.0);
    }

    pub fn set_phase_duties(&mut self, a: f32, b: f32, c: f32) {
        const PWM_MAX: u16 = 4095;
        let to_counts = |u: f32| -> u16 { (u.clamp(0.0, 1.0) * PWM_MAX as f32) as u16 };

        let duties = [a, b, c];

        // Handle each phase separately due to different const generic types
        for (i, &duty) in duties.iter().enumerate() {
            let (high_duty, low_duty) = match self.phase_states[i] {
                PhaseState::On => {
                    // Normal complementary PWM with deadtime
                    let high_duty = (duty - self.dead_zone).clamp(0.0, 1.0);
                    let low_duty = 1.0 - (duty + self.dead_zone).clamp(0.0, 1.0);
                    (to_counts(high_duty), to_counts(low_duty))
                }
                PhaseState::High => {
                    // Only high-side active
                    (to_counts(duty), 0)
                }
                PhaseState::Low => {
                    // Only low-side active
                    (0, to_counts(duty))
                }
                PhaseState::Off => {
                    // Both switches off
                    (0, 0)
                }
            };

            match i {
                0 => {
                    self.uh.set_duty_cycle(high_duty);
                    self.ul.set_duty_cycle(low_duty);
                }
                1 => {
                    self.vh.set_duty_cycle(high_duty);
                    self.vl.set_duty_cycle(low_duty);
                }
                2 => {
                    self.wh.set_duty_cycle(high_duty);
                    self.wl.set_duty_cycle(low_duty);
                }
                _ => unreachable!(),
            }
        }
    }
}
