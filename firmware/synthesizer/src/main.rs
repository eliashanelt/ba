#![no_std]
#![no_main]

use core::mem::MaybeUninit;
use cortex_m_rt::entry;
use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::{init_secondary, SharedData};
use embassy_stm32::{Config, Peripherals};
use embassy_time::Delay;
use embedded_hal_async::delay::DelayNs;
#[cfg(feature = "debug")]
use {defmt_rtt as _, panic_probe as _};

#[link_section = ".ram_d3"]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Configure the microcontroller clocks, etc.
    let mut config = Config::default();
    // Adjust for the H7’s clock, supply, etc. if needed
    // e.g., config.rcc...
    let p = embassy_stm32::init_primary(config, &SHARED_DATA);
    // Example: blink an LED on a known Portenta pin
    let mut delay = Delay;
    loop {
        // toggle LED
        // e.g. some pin.set_high();
        delay.delay_ms(500).await;
        info!("Hello World!");
        // e.g. pin.set_low();
        delay.delay_ms(500).await;
    }
}
