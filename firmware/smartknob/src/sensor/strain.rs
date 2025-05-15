use embassy_time::{Duration, Timer};
use embedded_hal::digital::{InputPin, OutputPin};

/// Simple async HX711 driver
pub struct Hx711<CLK, DOUT> {
    clk: CLK,
    dout: DOUT,
}

impl<CLK, DOUT> Hx711<CLK, DOUT>
where
    CLK: OutputPin, // PD_SCK
    DOUT: InputPin, // DOUT
{
    /// PD_SCK must start low.
    pub fn new(mut clk: CLK, dout: DOUT) -> Self {
        // ensure clock is low
        clk.set_low().ok();
        // optionally configure EXTI on dout for interrupts:
        // dout.make_interrupt_source(...); dout.enable_interrupt(...);
        Self { clk, dout }
    }

    /// Read one raw sample (awaits data‐ready, then clocks out 24 bits + 1 extra pulse).
    pub async fn read_raw(&mut self) -> i32 {
        // 1) wait for DOUT to go low (data ready)
        while self.dout.is_high().unwrap() {
            // poll every 10 µs
            Timer::after(Duration::from_micros(10)).await;
        }

        // 2) clock out 24 bits MSB-first
        let mut count: i32 = 0;
        for _ in 0..24 {
            self.clk.set_high().ok();
            Timer::after(Duration::from_micros(1)).await;
            count = (count << 1) | if self.dout.is_high().unwrap() { 1 } else { 0 };
            self.clk.set_low().ok();
            Timer::after(Duration::from_micros(1)).await;
        }

        // 3) one extra pulse to set gain/channel (default 128 gain)
        self.clk.set_high().ok();
        Timer::after(Duration::from_micros(1)).await;
        self.clk.set_low().ok();

        // 4) convert from 24-bit two’s-complement to i32
        if (count & 0x8000_00) != 0 {
            count |= !0xFF_FFFF;
        }
        count
    }
}
