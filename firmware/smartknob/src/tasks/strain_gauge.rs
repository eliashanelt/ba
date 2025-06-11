use embassy_time::{Duration, Instant, Ticker, Timer};
use esp_hal::{
    gpio::{Input, Output},
    time::Rate,
};

use crate::{
    sensor::strain::Hx711,
    util::{lerp, rate_to_duration, MovingAverage},
};

const STRAIN_PRESSED: f32 = 1.0;
const PRESS_WEIGHT: i32 = 400_000;
const POLLING_RATE: Rate = Rate::from_hz(120);
const LONG_PRESS_DURATION: Duration = Duration::from_millis(20);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    let mut strain_filter = MovingAverage::<10>::new();
    let mut virtual_button_state: VirtualButtonState = VirtualButtonState::Idle;
    let mut last_press_triggered_at = Instant::now();
    loop {
        ticker.next().await;
        let now = Instant::now();
        let raw = hx711.read_raw().await;
        let raw_avg = strain_filter.add(raw as f32);
        let press_value = lerp(raw_avg, 0.0, PRESS_WEIGHT as f32, 0.0, 1.0);
        if press_value < STRAIN_PRESSED {
            //TODO: Play led ring and motor stuff
            virtual_button_state = match virtual_button_state {
                VirtualButtonState::ShortPressed => VirtualButtonState::ShortPressReleased,
                VirtualButtonState::LongPressed => VirtualButtonState::LongPressReleased,
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
