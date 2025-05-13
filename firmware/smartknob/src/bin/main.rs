#![no_std]
#![no_main]

use core::fmt::DebugList;
use core::slice::IterMut;
use defmt::info;
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};
use embassy_time::{Delay, Instant, Ticker};
use embassy_time::{Duration, Timer};
use embedded_hal::delay::DelayNs;
use esp_hal::clock::CpuClock;
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::gpio::{Level, Output, OutputConfig, Pull};
use esp_hal::i2c::master::I2c;
use esp_hal::spi::master::Spi;
use esp_hal::time::Rate;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::{dma_buffers, i2c, spi};
use panic_rtt_target as _;

use smartknob::motor::mt6701::Mt6701;

const LED_COUNT: usize = 72;
const CLK_HZ: u64 = 240_000_000;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.3.1

    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let p = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timer0 = SystemTimer::new(p.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    let led_data = Output::new(p.GPIO12, Level::High, OutputConfig::default());

    let sclk = p.GPIO13;
    let miso = p.GPIO14;
    let mosi = p.GPIO3;
    let cs = p.GPIO11;

    let spi = Spi::new(
        p.SPI2,
        spi::master::Config::default()
            .with_frequency(Rate::from_mhz(4))
            .with_mode(spi::Mode::_1),
    )
    .unwrap()
    .with_sck(sclk)
    .with_mosi(mosi)
    .with_miso(miso)
    .with_cs(cs);

    let mut mt6701 = Mt6701 {
        spi,
        x: 0.0,
        y: 0.0,
        error: None,
    };

    let sda = p.GPIO39;
    let scl = p.GPIO38;

    let i2c = I2c::new(
        p.I2C0,
        i2c::master::Config::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(sda)
    .with_scl(scl)
    .into_async();

    let tmc_wl = p.GPIO7;
    let tmc_uh = p.GPIO8;
    let tmc_wl = p.GPIO15;
    let tmc_ul = p.GPIO16;
    let tmc_wh = p.GPIO17;
    let tmc_vh = p.GPIO18;

    info!("Embassy initialized!");

    spawner.spawn(led_ring(led_data)).unwrap();
    let mut ticker = Ticker::every(Duration::from_micros(100));

    loop {
        let angle = mt6701.read_angle().await;
        info!("angle: {}", angle);
        ticker.next().await;
    }

    //let mut led_ring_channel = Channel::<NoopRawMutex, u32, 1>::new();
    //spawner.must_spawn(tasks::led_ring());
}

#[embassy_executor::task]
pub async fn led_ring(mut led_data: Output<'static>) {
    let mut leds = [RGB8::default(); LED_COUNT];
    let mut increasing = true;
    let mut count = 0;
    loop {
        for hue in 0u8..=255 {
            let rgb = hsv_to_rgb(hue);
            let (on, off) = leds.split_at_mut(count.min(LED_COUNT));
            on.fill(rgb);
            off.fill(RGB8::default());
            write_sk6805(&mut led_data, &leds);
            Timer::after(Duration::from_millis(4)).await;
            if increasing {
                count += 1;
                if count == LED_COUNT {
                    increasing = false;
                }
            } else {
                count -= 1;
                if count == 0 {
                    increasing = true;
                }
            }
        }
    }
}

#[inline(always)]
fn delay_cycles(cycles: u32) {
    esp_hal::xtensa_lx::timer::delay(cycles);
}

#[inline(always)]
fn send_bit(pin: &mut Output<'static>, bit_is_one: bool) {
    // ********  timing constants from the datasheet  ********
    const T0H_NS: u32 = ns_to_cycles(300); // 0-code high   (0.3 µs)
    const T0L_NS: u32 = ns_to_cycles(900); // 0-code low    (0.6 µs)
    const T1H_NS: u32 = ns_to_cycles(600); // 1-code high   (0.6 µs)
    const T1L_NS: u32 = ns_to_cycles(600); // 1-code low    (0.6 µs)

    // ********  generate the waveform  ********
    critical_section::with(|_| {
        pin.set_high();
        if bit_is_one {
            delay_cycles(T1H_NS); // “1” – stay high a bit longer
            pin.set_low();
            delay_cycles(T1L_NS);
        } else {
            delay_cycles(T0H_NS); // “0” – short high pulse
            pin.set_low();
            delay_cycles(T0L_NS);
        }
    });
}

const fn ns_to_cycles(ns: u32) -> u32 {
    (((CLK_HZ) * ns as u64 + 999_999_999) / 1_000_000_000) as u32
}

fn send_byte(pin: &mut Output<'static>, mut byte: u8) {
    for _ in 0..8 {
        send_bit(pin, (byte & 0x80) != 0);
        byte <<= 1;
    }
}

fn hsv_to_rgb(h: u8) -> RGB8 {
    let region = h / 43; // 6 regions over 0–255
    let remainder = (h as u16 - region as u16 * 43) * 6;

    let p = 0;
    let q = (255 * 255 - 255 * remainder / 256) as u8;
    let t = (255 * remainder / 256) as u8;

    match region {
        0 => RGB8 { r: 255, g: t, b: p },
        1 => RGB8 { r: q, g: 255, b: p },
        2 => RGB8 { r: p, g: 255, b: t },
        3 => RGB8 { r: p, g: q, b: 255 },
        4 => RGB8 { r: t, g: p, b: 255 },
        _ => RGB8 { r: 255, g: p, b: q },
    }
}

#[derive(Default, Clone, Copy, defmt::Format, Debug)]
struct RGB8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

fn write_sk6805(pin: &mut Output<'static>, frame: &[RGB8]) {
    // Iterate over each byte **in GRB order**
    for rgb in frame {
        for &byte in &[rgb.g, rgb.r, rgb.b] {
            send_byte(pin, byte);
        }
    }
    Delay.delay_us(80);
}
