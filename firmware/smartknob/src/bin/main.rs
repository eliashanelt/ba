#![no_std]
#![no_main]

use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, Ordering};

use alloc::format;
use critical_section::Mutex;
use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Delay, Duration, Ticker, Timer};
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::gpio::{Event, Input, InputConfig, Io, Pull};
use esp_hal::i2c::master::I2c;
use esp_hal::interrupt::{InterruptHandler, Priority};
use esp_hal::spi::Mode;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{clock::CpuClock, spi::master::Spi};
use esp_hal::{dma_buffers, i2c, ram};
use esp_hal::{handler, spi};
use rotary_encoder_embedded::RotaryEncoder;
use smartknob::lcd::{self, Backlight, Direction, Display};
use {esp_backtrace as _, esp_println as _};

extern crate alloc;

const LCD_SCREEN_ADDRESS: u8 = 0x27;
pub const GAIN_RANGE: core::ops::RangeInclusive<i32> = -12..=12; // [dB]
pub const AMP_RANGE: core::ops::RangeInclusive<i32> = 0..=2000; // [mV]
pub const DELAY_RANGE: core::ops::RangeInclusive<i32> = 0..=5000; // [ms]
pub const PUMP_RANGE: core::ops::RangeInclusive<i32> = -1000..=1000; // [mBar]

static BUTTON: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

static SETTINGS: Mutex<RefCell<Settings>> = Mutex::new(RefCell::new(Settings::DEFAULT);
static CURRENT_SETTING_SELECTION: Mutex<RefCell<SettingSelection>> =
    Mutex::new(RefCell::new(SettingSelection::Gain));
static BUTTON_PRESSED: AtomicBool = AtomicBool::new(false);

#[derive(Copy, Clone, Debug, PartialEq)]
enum SettingSelection {
    Gain,
    Amplitude,
    Delay,
    Pressure,
}

impl SettingSelection {
    pub fn next(&mut self) {
        *self = match self {
            Self::Gain => Self::Amplitude,
            Self::Amplitude => Self::Delay,
            Self::Delay => Self::Pressure,
            Self::Pressure => Self::Gain,
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            Self::Gain => "db",
            Self::Amplitude => "mV",
            Self::Delay => "ms",
            Self::Pressure => "mBar",
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Settings {
    gain: i32,                //[dB]
    amplitude_threshold: i32, //[mV]
    delay: i32,               //[ms]
    pressure: i32,            //[mBar]
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let p = esp_hal::init(config);
    let mut io = Io::new(p.IO_MUX);
    io.set_interrupt_handler(handler);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timer0 = TimerGroup::new(p.TIMG1);
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

    let sda = p.GPIO18;
    let scl = p.GPIO19;

    let i2c = I2c::new(p.I2C0, i2c::master::Config::default())
        .unwrap()
        .with_sda(sda)
        .with_scl(scl)
        .into_async();

    let mut lcd = lcd::Lcd::new(i2c, lcd::ADDRESS, Delay).await.unwrap();

    lcd.set_display(Display::On).await.unwrap();
    lcd.set_backlight(Backlight::On).await.unwrap();

    let rotary_clk: Input<'static> =
        Input::new(p.GPIO32, InputConfig::default().with_pull(Pull::Up));
    let rotary_dt: Input<'static> =
        Input::new(p.GPIO35, InputConfig::default().with_pull(Pull::Up));
    let mut button = Input::new(p.GPIO34, InputConfig::default().with_pull(Pull::Up));
    critical_section::with(|cs| {
        button.listen(Event::FallingEdge);
        BUTTON.borrow_ref_mut(cs).replace(button)
    });

    // 4) Spawn two independent tasks
    spawner.spawn(encoder_task(rotary_dt, rotary_clk)).unwrap();

    let mut ticker = Ticker::every(Duration::from_millis(100));
    let mut setting_selection = SettingSelection::Gain;
    lcd.clear().await.unwrap();
    lcd.print(&format!("{setting_selection:?}")).await.unwrap();

    loop {
        ticker.next().await;
        if BUTTON_PRESSED.swap(false, Ordering::AcqRel) {
            setting_selection.next();
            lcd.clear().await.unwrap();
            lcd.print(&format!("{setting_selection:?}")).await.unwrap();
        }
    }
}

#[handler]
#[ram]
fn handler() {
    critical_section::with(|cs| {
        if let Some(btn) = BUTTON.borrow_ref_mut(cs).as_mut() {
            btn.clear_interrupt();
        }
    });
    BUTTON_PRESSED.store(true, Ordering::SeqCst);
}

#[embassy_executor::task]
async fn encoder_task(dt: Input<'static>, clk: Input<'static>) {
    let mut encoder = RotaryEncoder::new(dt, clk).into_standard_mode();

    // 900 Hz ticker ⇒ period ≈ 1.111 ms
    let mut ticker = Ticker::every(Duration::from_micros(1_111));

    use rotary_encoder_embedded::Direction;
    loop {
        ticker.next().await;
        match encoder.update() {
            Direction::Clockwise => {
                defmt::info!("Encoder → CW");
                // e.g. ++counter;
            }
            Direction::Anticlockwise => {
                defmt::info!("Encoder ← CCW");
                // e.g. --counter;
            }
            Direction::None => {}
        }
    }
}
