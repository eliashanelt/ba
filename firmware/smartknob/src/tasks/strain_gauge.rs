use defmt::info;
use embassy_time::{Duration, Instant, Ticker};
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
    let mut ticker = Ticker::every(Duration::from_millis(
        POLLING_RATE.as_duration().as_millis(),
    ));
    let mut last = Instant::now();
    loop {
        ticker.next().await;
        let now = Instant::now();
        let dt = now.duration_since(last).as_millis();
        last = now;
        let strain = hx711.read_raw().await;
        if strain > PRESS_THRESHOLD {
            info!("press detected");
        }
        ticker.next().await;
    }
}

#[derive(Debug, Clone)]
struct MovingAverage<const SIZE: usize> {
    buf: [f32; SIZE],
    sum: f32,
    idx: usize,
    count: usize,
}

impl<const SIZE: usize> MovingAverage<SIZE> {
    /// Create a new empty mover over a zero‐initialized buffer.
    pub fn new() -> Self {
        Self {
            buf: [0.0; SIZE],
            sum: 0.0,
            idx: 0,
            count: 0,
        }
    }

    /// Push a new value, return the current average.
    pub fn add(&mut self, value: f32) -> f32 {
        // subtract the oldest, add the new
        let old = self.buf[self.idx];
        self.sum += value;
        self.sum -= old;
        self.buf[self.idx] = value;

        // advance the circular index
        self.idx = (self.idx + 1) % SIZE;
        if self.count < SIZE {
            self.count += 1;
        }

        self.sum / self.count as f32
    }
}
