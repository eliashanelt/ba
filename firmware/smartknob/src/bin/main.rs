#![no_std]
#![no_main]

use core::slice::IterMut;
use defmt::info;
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};
use embassy_time::Delay;
use embassy_time::{Duration, Timer};
use embedded_hal::delay::DelayNs;
// use embedded_hal_async::delay::DelayNs;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig, Pull};
use esp_hal::timer::systimer::SystemTimer;
use panic_rtt_target as _;

extern crate alloc;

//pub mod led_ring;
//pub mod motor;
//mod tasks;
//
const LED_COUNT: usize = 1;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.3.1

    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let p = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timer0 = SystemTimer::new(p.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    let mut led_data = Output::new(p.GPIO12, Level::High, OutputConfig::default());

    info!("Embassy initialized!");

    //let mut led_ring_channel = Channel::<NoopRawMutex, u32, 1>::new();
    //spawner.must_spawn(tasks::led_ring());

    let mut leds = [RGB8::default(); LED_COUNT];

    loop {
        Timer::after(Duration::from_micros(4000)).await;
        continue;
        for hue in 0u8..=255 {
            let rgb = hsv_to_rgb(hue);

            leds.fill(rgb);
            write_sk6805(&mut led_data, &leds);

            Timer::after(Duration::from_micros(4000)).await;
        } //info!("Hello world!");
          //Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-beta.0/examples/src/bin
}

#[inline(always)]
fn send_bit(pin: &mut Output<'static>, bit_is_one: bool) {
    // ********  timing constants from the datasheet  ********
    const T0H_NS: u32 = 300; // 0-code high   (0.3 µs)
    const T0L_NS: u32 = 800; // 0-code low    (0.8 µs)
    const T1H_NS: u32 = 600; // 1-code high   (0.6 µs)
    const T1L_NS: u32 = 200; // 1-code low    (0.2 µs)

    // ********  generate the waveform  ********
    pin.set_high();

    if bit_is_one {
        Delay.delay_ns(T1H_NS); // “1” – stay high a bit longer
        pin.set_low();
        Delay.delay_ns(T1L_NS);
    } else {
        Delay.delay_ns(T0H_NS); // “0” – short high pulse
        pin.set_low();
        Delay.delay_ns(T0L_NS);
    }
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

#[derive(Default, Clone, Copy)]
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
