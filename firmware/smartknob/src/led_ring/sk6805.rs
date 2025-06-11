use embassy_time::Delay;
use embedded_hal::delay::DelayNs;
use esp_hal::gpio::Output;

use super::{rgb8::RGB8, CLK_HZ};

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

pub fn write(pin: &mut Output<'static>, frame: &[RGB8]) {
    for rgb in frame {
        for &b in &[rgb.g, rgb.r, rgb.b] {
            send_byte(pin, b);
        }
    }
    pin.set_low();
    Delay.delay_us(80);
}
