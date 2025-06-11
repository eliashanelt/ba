pub enum Effect {
    Snake,
    StaticColor,
    LightHouse,
    Trail,
    FadeIn,
    FadeOut,
    LedsOff,
    ToBrightness,
    Carousel { position: usize },
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
            effect: Effect::Carousel { position: 0 },
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_update) < WANDER_INTERVAL {
            return;
        }
        self.last_update = now;
        match effect {}
        self.position = (self.position + 1) % LED_COUNT;
        let hue = ((self.position as f32 / LED_COUNT as f32) * 255.0) as u8;
        for i in 0..LED_COUNT {
            let raw_diff = if i >= self.position {
                i - self.position
            } else {
                LED_COUNT + i - self.position
            };
            let dist = raw_diff.min(LED_COUNT - raw_diff);

            if dist <= FADE_STEPS {
                let b = 1.0 - (dist as f32 / (FADE_STEPS + 2) as f32);
                self.leds[i] = hsv_to_rgb(hue) * b * b * b;
            } else {
                self.leds[i] = RGB8::default();
            }
        }

        write_sk6805(&mut self.led_pin, &self.leds);
    }
}
