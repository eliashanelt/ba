use defmt::info;
use embassy_time::{Duration, Ticker};
use esp_hal::{
    gpio::{Input, Output},
    time::Rate,
};

use crate::sensor::strain::Hx711;

const PRESS_THRESHOLD: i32 = 40000;
const STRAIN_RELEASED: f32 = 0.3;
const STRAIN_PRESSED: f32 = 1.0;
const POLLING_RATE: Rate = Rate::from_hz(120);

#[embassy_executor::task]
pub async fn strain_gauge(mut hx711: Hx711<Output<'static>, Input<'static>>) {
    let mut ticker = Ticker::every(POLLING_RATE.as_duration());
    loop {
        let strain = hx711.read_raw().await;
        if strain > PRESS_THRESHOLD {
            info!("PRESS_DETECTED");
        }
        ticker.next().await;
    }
}

struct MovingAverage<T, const SIZE: usize> {
    buf: [SIZE; T],
}
