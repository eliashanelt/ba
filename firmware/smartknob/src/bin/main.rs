#![no_std]
#![no_main]

use core::fmt::DebugList;
use core::slice::IterMut;

use core::ops::Mul;
use defmt::info;
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};
use embassy_time::{Delay, Instant, Ticker};
use embassy_time::{Duration, Timer};
use embedded_hal::delay::DelayNs;
use embedded_hal::pwm::SetDutyCycle;
use esp_hal::clock::CpuClock;
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::i2c::master::I2c;
use esp_hal::mcpwm::operator::PwmPinConfig;
use esp_hal::mcpwm::timer::PwmWorkingMode;
use esp_hal::mcpwm::{McPwm, PeripheralClockConfig};
use esp_hal::spi::master::Spi;
use esp_hal::time::Rate;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{dma_buffers, i2c, spi};
use panic_rtt_target as _;

use smartknob::motor::mt6701::Mt6701;
use smartknob::sensor::strain::Hx711;

const LED_COUNT: usize = 72;
const CLK_HZ: u64 = 240_000_000;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.3.1

    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let p = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let tg = TimerGroup::new(p.TIMG1);
    esp_hal_embassy::init(tg.timer0);

    let led_data = Output::new(p.GPIO12, Level::High, OutputConfig::default());

    let sclk = p.GPIO13;
    let miso = p.GPIO14;
    let mosi = p.GPIO3;
    let cs = p.GPIO11;

    let clock_cfg = PeripheralClockConfig::with_frequency(Rate::from_mhz(40)).unwrap();
    let mut mcpwm = McPwm::new(p.MCPWM0, clock_cfg);

    mcpwm.operator0.set_timer(&mcpwm.timer0);
    mcpwm.operator1.set_timer(&mcpwm.timer1);
    mcpwm.operator2.set_timer(&mcpwm.timer2);

    // Map pins: UH/A, UL/B; WH/A, WL/B; VH/A, VL/B

    let (mut uh, mut ul) = mcpwm.operator0.with_pins(
        p.GPIO8,
        PwmPinConfig::UP_ACTIVE_HIGH,
        p.GPIO16,
        PwmPinConfig::UP_ACTIVE_HIGH,
    );

    let (mut wh, mut wl) = mcpwm.operator1.with_pins(
        p.GPIO17,
        PwmPinConfig::UP_ACTIVE_HIGH,
        p.GPIO7,
        PwmPinConfig::UP_ACTIVE_HIGH,
    );

    let (mut vh, mut vl) = mcpwm.operator2.with_pins(
        p.GPIO18,
        PwmPinConfig::UP_ACTIVE_HIGH,
        p.GPIO15,
        PwmPinConfig::UP_ACTIVE_HIGH,
    ); // Configure all timers for 20 kHz, 12-bit resolution (0..=4095)
    let timer_cfg = clock_cfg
        .timer_clock_with_frequency(1599, PwmWorkingMode::Increase, Rate::from_khz(20))
        .unwrap();
    mcpwm.timer0.start(timer_cfg);
    mcpwm.timer1.start(timer_cfg);
    mcpwm.timer2.start(timer_cfg);

    let seq = [
        (4095, 0, 0, 4095, 0, 0), // U+, V-, W off
        (4095, 0, 0, 0, 0, 4095), // ...
        (0, 4095, 0, 0, 4095, 0),
        (0, 4095, 0, 0, 0, 4095),
        (0, 0, 4095, 4095, 0, 0),
        (4095, 0, 4095, 0, 0, 0),
    ];
    let step_time = Duration::from_millis(2);

    loop {
        for &(u_h, u_l, w_h, w_l, v_h, v_l) in &seq {
            uh.set_duty_cycle(u_h);
            ul.set_duty_cycle(u_l);
            wh.set_duty_cycle(w_h);
            wl.set_duty_cycle(w_l);
            vh.set_duty_cycle(v_h);
            vl.set_duty_cycle(v_l);
            Timer::after(step_time).await;
        }
    }

    return;

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

    //let tmc_wl = p.GPIO7;
    //let tmc_uh = p.GPIO8;
    //let tmc_wl = p.GPIO15;
    //let tmc_ul = p.GPIO16;
    //let tmc_wh = p.GPIO17;
    //let tmc_vh = p.GPIO18;

    info!("Embassy initialized!");

    let clk = Output::new(p.GPIO1, Level::Low, OutputConfig::default());
    let dout = Input::new(p.GPIO21, InputConfig::default());
    let mut hx = Hx711::new(clk, dout);

    spawner.spawn(led_ring(led_data)).unwrap();
    let mut ticker = Ticker::every(Duration::from_secs(1));

    loop {
        let angle = mt6701.read_angle().await;
        let strain = hx.read_raw().await;
        info!("angle: {}, strain: {}", angle, strain);
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
            let rgb = hsv_to_rgb(hue) * 0.05;
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

impl Mul<f32> for RGB8 {
    type Output = RGB8;

    fn mul(self, scalar: f32) -> RGB8 {
        fn scale_channel(channel: u8, scalar: f32) -> u8 {
            let scaled = channel as f32 * scalar;
            scaled.clamp(0.0, 255.0) as u8
        }

        RGB8 {
            r: scale_channel(self.r, scalar),
            g: scale_channel(self.g, scalar),
            b: scale_channel(self.b, scalar),
        }
    }
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
