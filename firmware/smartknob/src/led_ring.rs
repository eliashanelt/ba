use core::{
    cell::Cell,
    ops::{Add, Mul},
};

use embassy_time::{Delay, Duration, Instant};
use embedded_hal::delay::DelayNs;
use esp_hal::gpio::Output;

const LED_COUNT: usize = 72;
const CLK_HZ: u64 = 240_000_000;

pub enum Effect {
    Snake,
    StaticColor,
    LightHouse,
    Trail,
    FadeIn,
    FadeOut,
    LedsOff,
    ToBrightness,
    Carousel { position: Cell<usize> },
}

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
    pub const RED: Self = RGB8 { r: 255, b: 0, g: 0 };
}

impl Mul<f32> for RGB8 {
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
}

impl Add<RGB8> for RGB8 {
    type Output = RGB8;

    fn add(self, rhs: RGB8) -> Self::Output {
        RGB8 {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
        }
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

const FADE_STEPS: usize = 6;
// how fast the bright pixel moves (e.g. every 100 ms)
const WANDER_INTERVAL: Duration = Duration::from_millis(40);

pub struct LedRing {
    leds: [RGB8; LED_COUNT],
    led_pin: Output<'static>,
    last_update: Instant,
    effect: Effect,
}

impl LedRing {
    pub fn new(led_pin: Output<'static>) -> Self {
        let leds = [RGB8::default(); LED_COUNT];
        Self {
            leds,
            led_pin,
            last_update: Instant::now(),
            effect: Effect::Carousel {
                position: Cell::new(0),
            },
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_update) < WANDER_INTERVAL {
            return;
        }
        self.last_update = now;
        match &self.effect {
            Effect::Carousel { position } => {
                position.set((position.get() + 1) % LED_COUNT);
                let position = position.get();
                let hue = ((position as f32 / LED_COUNT as f32) * 255.0) as u8;
                for i in 0..LED_COUNT {
                    let raw_diff = if i >= position {
                        i - position
                    } else {
                        LED_COUNT + i - position
                    };
                    let dist = raw_diff.min(LED_COUNT - raw_diff);

                    if dist <= FADE_STEPS {
                        let b = 1.0 - (dist as f32 / (FADE_STEPS + 2) as f32);
                        self.leds[i] = hsv_to_rgb(hue) * b * b * b;
                    } else {
                        self.leds[i] = RGB8::default();
                    }
                }
            }
            _ => {}
        }

        write_sk6805(&mut self.led_pin, &self.leds);
    }
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
    pin.set_low();
    Delay.delay_us(80);
}
