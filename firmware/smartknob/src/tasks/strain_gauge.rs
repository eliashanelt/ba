use core::fmt::write;

use defmt::info;
use embassy_time::{Duration, Instant, Ticker, Timer};
use esp_hal::{
    gpio::{Input, Output},
    time::Rate,
};

use crate::{
    sensor::strain::Hx711,
    util::{lerp, rate_to_duration, MovingAverage},
};

use super::motor::MOTOR_CH;

const STRAIN_PRESSED: f32 = 400_000.0;
const PRESS_WEIGHT: i32 = 400_000;
const POLLING_RATE: Rate = Rate::from_hz(240);
const LONG_PRESS_DURATION: Duration = Duration::from_millis(20);

#[derive(Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum VirtualButtonState {
    LongPressReleased,
    ShortPressReleased,
    ShortPressed,
    LongPressed,
    Idle,
}

#[embassy_executor::task]
pub async fn strain_gauge(mut hx711: Hx711<Output<'static>, Input<'static>>) {
    let mut ticker = Ticker::every(rate_to_duration(POLLING_RATE));
    let mut strain_filter = MovingAverage::<5>::new();
    let mut virtual_button_state: VirtualButtonState = VirtualButtonState::Idle;
    let mut last_press_triggered_at = Instant::now();
    let motor_ch_sender = MOTOR_CH.sender();
    loop {
        ticker.next().await;
        let now = Instant::now();
        let raw = hx711.read_raw().await;
        if raw < 0 || raw > 2_000_000 {
            continue;
        }
        let raw_avg = strain_filter.add(raw as f32);
        let press_value = raw_avg;
        //info!("button state: {}, {}", press_value, raw);
        info!("{} | {}", virtual_button_state, press_value);
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
