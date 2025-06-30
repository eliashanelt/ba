use core::fmt::write;

use defmt::info;
use embassy_time::{Delay, Duration, Instant, Ticker, Timer};
use esp_hal::{
    gpio::{Input, Output},
    time::Rate,
};

use crate::{
    sensor::strain::HX711,
    util::{lerp, rate_to_duration, MovingAverage},
};

use super::motor::MOTOR_CH;

const STRAIN_PRESSED: f32 = 10_000.0;
const POLLING_RATE: Rate = Rate::from_hz(120);
const LONG_PRESS_DURATION: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum VirtualButtonState {
    LongPressReleased,
    ShortPressReleased,
    ShortPressed,
    LongPressed,
    Idle,
}

#[embassy_executor::task]
pub async fn strain_gauge(mut hx711: HX711<Output<'static>, Input<'static>, Delay>) {
    let mut ticker = Ticker::every(rate_to_duration(POLLING_RATE));
    let mut strain_filter = MovingAverage::<5>::new();
    let mut virtual_button_state: VirtualButtonState = VirtualButtonState::Idle;
    let mut last_press_triggered_at = Instant::now();
    let motor_ch_sender = MOTOR_CH.sender();
    hx711.tare(16);
    loop {
        ticker.next().await;
        let now = Instant::now();
        if !hx711.is_ready() {
            continue;
        }
        let Ok(raw) = hx711.read() else {
            continue;
        };
        Timer::after(Duration::from_millis(100)).await;
        Timer::after(Duration::from_millis(100)).await;
        let press_value = strain_filter.add(raw as f32);
        if press_value < STRAIN_PRESSED {
            //TODO: Play led ring and motor stuff
            virtual_button_state = match virtual_button_state {
                VirtualButtonState::ShortPressed => {
                    motor_ch_sender
                        .send(super::motor::Command::Haptic(super::motor::Press::Short))
                        .await;
                    VirtualButtonState::ShortPressReleased
                }
                VirtualButtonState::LongPressed => {
                    motor_ch_sender
                        .send(super::motor::Command::Haptic(super::motor::Press::Long))
                        .await;
                    VirtualButtonState::LongPressReleased
                }
                _ => {
                    Timer::after(Duration::from_millis(20)).await;
                    VirtualButtonState::Idle
                }
            };
            last_press_triggered_at = now;
        } else if STRAIN_PRESSED < press_value
            && virtual_button_state == VirtualButtonState::ShortPressed
            && now.duration_since(last_press_triggered_at) >= LONG_PRESS_DURATION
        {
            virtual_button_state = VirtualButtonState::LongPressed;
        } else if press_value > STRAIN_PRESSED && virtual_button_state == VirtualButtonState::Idle {
            virtual_button_state = VirtualButtonState::ShortPressed
        }
    }
}
