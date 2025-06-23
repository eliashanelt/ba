use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Ticker};
use esp_hal::gpio::Output;

use crate::led_ring::LedRing;

static TRIGGER: Signal<CriticalSectionRawMutex, ()> = Signal::new();

#[embassy_executor::task]
pub async fn led_ring(mut led_pin: Output<'static>) {
    let mut led_ring = LedRing::new(led_pin);
    let mut ticker = Ticker::every(Duration::from_millis(4)); // 100 Hz
    loop {
        led_ring.update();
        ticker.next().await
    }
}
