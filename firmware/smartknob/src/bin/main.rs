#![no_std]
#![no_main]

use esp_hal::clock::CpuClock;
use esp_hal::timer::systimer::SystemTimer;

 use defmt::{info, error, warn};

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use panic_rtt_target as _;

extern crate alloc;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.3.1

    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    info!("Embassy initialized!");

    // TODO: Spawn some tasks
    let _ = spawner;

    let mut counter = 0;
    loop {
        info!("Hello worldA!");
        warn!("WAs will er");
        error!("KAKKAKKA {}", counter);
        Timer::after(Duration::from_secs(1)).await;
        counter += 1;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-beta.0/examples/src/bin
}
