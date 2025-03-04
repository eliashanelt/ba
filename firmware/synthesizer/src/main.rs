#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::{
    adc::{Adc, AdcChannel, Resolution, SampleTime},
    Config,
};
use embassy_time::Delay;
use embedded_hal_async::delay::DelayNs;
#[cfg(feature = "debug")]
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let config = Config::default();
    let mut peripherals = embassy_stm32::init(config);
    let mut adc1 = Adc::new(peripherals.ADC1);
    adc1.set_resolution(Resolution::BITS16);
    let mut adc_pin0 = peripherals.PA0.degrade_adc();
    let mut adc_pin2 = peripherals.PA2.degrade_adc();
    let mut measurements = [0u16; 2];
    let mut delay = Delay;
    loop {
        adc1.read(
            &mut peripherals.DMA2_CH0,
            [
                (&mut adc_pin0, SampleTime::CYCLES1_5),
                (&mut adc_pin2, SampleTime::CYCLES1_5),
            ]
            .into_iter(),
            &mut measurements,
        )
        .await;
        defmt::info!("measurements: {}", measurements);
        delay.delay_ms(500).await;
    }
}
