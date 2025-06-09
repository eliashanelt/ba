#![no_std]
#![no_main]

//! Modified on 2025-06-09
//! – Spins the BLDC at **0.25 Hz** (≈4 s per mechanical rev)
//! – LED ring shows the **absolute rotor angle** reported by the MT6701 *in real-time*,
//!   so if you stall the shaft by hand the LEDs freeze accordingly.
//!
//! The rest of the application remains identical to your original example,
//! except that everything now happens in one task – no `Channel` is needed.

use core::f32::consts::PI;
use core::ops::Mul;
use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Ticker, Timer};
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::mcpwm::operator::{PwmPin, PwmPinConfig};
use esp_hal::mcpwm::timer::PwmWorkingMode;
use esp_hal::mcpwm::{McPwm, PeripheralClockConfig};
use esp_hal::peripherals::MCPWM0;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{i2c, spi};
use panic_rtt_target as _;

use smartknob::motor::mt6701::Mt6701;
use smartknob::sensor::strain::Hx711;

// ======== Application-wide constants ========================================

const LED_COUNT: usize = 72;
const CLK_HZ: u64 = 240_000_000;

const POLE_PAIRS: u32 = 8;
/// Mechanical period for 0.25 Hz = 4 000 000 µs
const REV_PERIOD_US: u32 = 4_000_000;
/// Commutation step every electrical 60 °
const STEP_US: u32 = REV_PERIOD_US / (6 * POLE_PAIRS); // = 83_333 µs

// ======== Entry-point =======================================================

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(esp_hal::clock::CpuClock::max());
    let p = esp_hal::init(config);

    // --- Memory heap (unchanged) -------------------------------------------
    esp_alloc::heap_allocator!(size: 72 * 1024);

    // --- Timer for Embassy --------------------------------------------------
    let tg = TimerGroup::new(p.TIMG1);
    esp_hal_embassy::init(tg.timer0);

    // --- LED data pin -------------------------------------------------------
    let mut led_data = Output::new(p.GPIO12, Level::High, OutputConfig::default());

    // --- MCPWM setup (unchanged, just shorter) -----------------------------
    let clock_cfg = PeripheralClockConfig::with_frequency(Rate::from_mhz(40)).unwrap();
    let mut mcpwm = McPwm::new(p.MCPWM0, clock_cfg);

    mcpwm.operator0.set_timer(&mcpwm.timer0);
    mcpwm.operator1.set_timer(&mcpwm.timer1);
    mcpwm.operator2.set_timer(&mcpwm.timer2);

    // Map BLDC phases: UH/UL, WH/WL, VH/VL ----------------------------------
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
    );

    // 20 kHz PWM, 12-bit resolution -----------------------------------------
    let timer_cfg = clock_cfg
        .timer_clock_with_frequency(1599, PwmWorkingMode::Increase, Rate::from_khz(20))
        .unwrap();
    mcpwm.timer0.start(timer_cfg);
    mcpwm.timer1.start(timer_cfg);
    mcpwm.timer2.start(timer_cfg);

    // --- MT6701 absolute encoder -------------------------------------------
    let spi = spi::master::Spi::new(
        p.SPI2,
        spi::master::Config::default()
            .with_frequency(Rate::from_mhz(4))
            .with_mode(spi::Mode::_1),
    )
    .unwrap()
    .with_sck(p.GPIO13)
    .with_mosi(p.GPIO3)
    .with_miso(p.GPIO14)
    .with_cs(p.GPIO11);

    let mut mt6701 = Mt6701 {
        spi,
        x: 0.0,
        y: 0.0,
        error: None,
    };

    // --- HX711 strain gauge (unchanged) ------------------------------------
    let clk = Output::new(p.GPIO1, Level::Low, OutputConfig::default());
    let dout = Input::new(p.GPIO21, InputConfig::default());
    let mut hx = Hx711::new(clk, dout);

    // ------------- Spawn the motor-drive task  ------------------------------
    /*spawner
    .spawn(open_loop_spin(uh, ul, wh, wl, vh, vl))
    .unwrap();*/

    // ­------------ Unified sampling + LED loop ------------
    let mut leds = [RGB8::default(); LED_COUNT];
    let mut ticker = Ticker::every(Duration::from_millis(4)); // 100 Hz
    loop {
        // ----- Acquire sensor data -----------------------------------------
        let angle_deg = mt6701.read_angle().await; // 0–360°
        let strain = hx.read_raw().await;

        // ----- Update LED ring ---------------------------------------------
        // Map 0-360° → 0-71 index.
        let idx = ((((angle_deg / (2.0 * PI)) * LED_COUNT as f32 * 2.0) + 1.0) / 2.0) as usize
            % LED_COUNT;

        if strain > 400000 {
            leds.fill(RGB8::WHITE);
        } else {
            leds.fill(RGB8::default());
            leds[idx] = RGB8 {
                r: 255,
                g: 40,
                b: 0,
            };
        }
        write_sk6805(&mut led_data, &leds);

        // ----- Logging ------------------------------------------------------
        /*info!(
            "angle: {}, strain: {}, led_index: {}",
            angle_deg, strain, idx
        );*/

        ticker.next().await; // keep loop at 100 Hz
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

// ======== Task: open-loop BLDC commutation (unchanged) ======================

#[embassy_executor::task]
async fn open_loop_spin(
    mut uh: PwmPin<'static, MCPWM0, 0, true>,
    mut ul: PwmPin<'static, MCPWM0, 0, false>,
    mut vh: PwmPin<'static, MCPWM0, 1, true>,
    mut vl: PwmPin<'static, MCPWM0, 1, false>,
    mut wh: PwmPin<'static, MCPWM0, 2, true>,
    mut wl: PwmPin<'static, MCPWM0, 2, false>,
) {
    use embassy_time::Ticker;

    // Six-step (trapezoidal) table ─ unchanged
    const TABLE: [[u16; 6]; 6] = [
        [4095, 0, 0, 0, 0, 4095], // A+ C-
        [4095, 0, 0, 0, 4095, 0], // A+ B-
        [0, 0, 0, 4095, 4095, 0], // B+ C-
        [0, 4095, 0, 4095, 0, 0], // B+ A-
        [0, 4095, 4095, 0, 0, 0], // C+ A-
        [0, 0, 4095, 0, 0, 4095], // C+ B-
    ];

    let mut step = 0usize;
    let mut ticker = Ticker::every(Duration::from_micros(STEP_US as _));

    loop {
        let phase = TABLE[step];
        uh.set_duty_cycle(phase[0]);
        ul.set_duty_cycle(phase[1]);
        vh.set_duty_cycle(phase[2]);
        vl.set_duty_cycle(phase[3]);
        wh.set_duty_cycle(phase[4]);
        wl.set_duty_cycle(phase[5]);

        step = (step + 1) % 6;
        ticker.next().await;
    }
}
// ======== Utility functions / structs (unchanged) ===========================

use embassy_time::Delay;
use embedded_hal::delay::DelayNs;
use embedded_hal::pwm::SetDutyCycle;

#[derive(Clone, Default, Copy, defmt::Format, Debug)]
struct RGB8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RGB8 {
    pub const WHITE: Self = RGB8 {
        r: 255,
        b: 255,
        g: 255,
    };
}

/*impl Mul<f32> for RGB8 {
    type Output = RGB8;
    fn mul(self, scalar: f32) -> RGB8 {
        fn sc(ch: u8, k: f32) -> u8 {
            (ch as f32 * k).clamp(0.0, 255.0) as u8
        }
        RGB8 {
            r: sc(self.r, scalar),
            g: sc(self.g, scalar),
            b: sc(self.b, scalar),
        }
    }
}*/

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

#[inline(always)]
fn delay_cycles(cycles: u32) {
    esp_hal::xtensa_lx::timer::delay(cycles);
}

#[inline(always)]
const fn ns_to_cycles(ns: u32) -> u32 {
    (((CLK_HZ) * ns as u64 + 999_999_999) / 1_000_000_000) as u32
}

fn send_byte(pin: &mut Output<'static>, mut byte: u8) {
    for _ in 0..8 {
        send_bit(pin, (byte & 0x80) != 0);
        byte <<= 1;
    }
}

fn write_sk6805(pin: &mut Output<'static>, frame: &[RGB8]) {
    for rgb in frame {
        for &b in &[rgb.g, rgb.r, rgb.b] {
            send_byte(pin, b);
        }
    }
    Delay.delay_us(80);
}
