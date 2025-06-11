pub mod rgb8;
pub mod sk6805;

use core::cell::Cell;

use embassy_time::{Duration, Instant};
use esp_hal::gpio::Output;
use rgb8::{hsv_to_rgb, RGB8};

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

        sk6805::write(&mut self.led_pin, &self.leds);
    }
}
