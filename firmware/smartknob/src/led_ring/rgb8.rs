use core::ops::{Add, Mul};

#[derive(Clone, Default, Copy, defmt::Format, Debug)]
pub struct RGB8 {
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
pub fn hsv_to_rgb(h: u8) -> RGB8 {
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
