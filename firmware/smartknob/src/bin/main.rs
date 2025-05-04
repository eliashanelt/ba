#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Delay, Duration, Timer};
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::i2c::master::I2c;
use esp_hal::spi;
use esp_hal::spi::Mode;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{clock::CpuClock, spi::master::Spi};
use esp_hal::{dma_buffers, i2c};
use smartknob::lcd::{self, Backlight, Display};
use {esp_backtrace as _, esp_println as _};

extern crate alloc;

const LCD_SCREEN_ADDRESS: u8 = 0x27;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timer0 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timer0.timer0);

    info!("Embassy initialized!");

    /*let dma_channel = peripherals.DMA_SPI2;

    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(32000);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    let sclk = peripherals.GPIO0;
    let miso = peripherals.GPIO2;
    let mosi = peripherals.GPIO4;
    let cs = peripherals.GPIO5;

    let mut spi = Spi::new(
        peripherals.SPI2,
        spi::master::Config::default()
            .with_frequency(Rate::from_khz(100))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sclk)
    .with_mosi(mosi)
    .with_miso(miso)
    .with_cs(cs)
    .with_dma(dma_channel)
    .with_buffers(dma_rx_buf, dma_tx_buf)
    .into_async();*/

    let sda = peripherals.GPIO18;
    let scl = peripherals.GPIO19;

    let i2c = I2c::new(peripherals.I2C0, i2c::master::Config::default())
        .unwrap()
        .with_sda(sda)
        .with_scl(scl)
        .into_async();

    let mut lcd = lcd::Lcd::new(i2c, lcd::ADDRESS, Delay).await.unwrap();

    lcd.set_display(Display::On).await.unwrap();
    lcd.set_backlight(Backlight::On).await.unwrap();

    lcd.clear().await.unwrap();
    lcd.print("Hello World!").await.unwrap();

    let _ = spawner;

    loop {
        info!("Hello world!");
        Timer::after(Duration::from_secs(1)).await;
        /*let send_buffer = [0, 1, 2, 3, 4, 5, 6, 7];
        let mut buffer = [0; 8];
        esp_println::println!("Sending bytes");
        embedded_hal_async::spi::SpiBus::transfer(&mut spi, &mut buffer, &send_buffer)
            .await
            .unwrap();
        esp_println::println!("Bytes received: {:?}", buffer);
        Timer::after(Duration::from_millis(5_000)).await;*/
    }
}
