use core::f32::consts::PI;
use esp_hal::{spi::master::Spi, Async, Blocking};

const ALPHA: f32 = 0.4;

// CRC6 lookup table
static TABLE_CRC6: [u8; 64] = [
    0x00, 0x03, 0x06, 0x05, 0x0C, 0x0F, 0x0A, 0x09, 0x18, 0x1B, 0x1E, 0x1D, 0x14, 0x17, 0x12, 0x11,
    0x30, 0x33, 0x36, 0x35, 0x3C, 0x3F, 0x3A, 0x39, 0x28, 0x2B, 0x2E, 0x2D, 0x24, 0x27, 0x22, 0x21,
    0x23, 0x20, 0x25, 0x26, 0x2F, 0x2C, 0x29, 0x2A, 0x3B, 0x38, 0x3D, 0x3E, 0x37, 0x34, 0x31, 0x32,
    0x13, 0x10, 0x15, 0x16, 0x1F, 0x1C, 0x19, 0x1A, 0x0B, 0x08, 0x0D, 0x0E, 0x07, 0x04, 0x01, 0x02,
];

fn crc6_43_18bit(input: u32) -> u8 {
    // right-aligned 18-bit CRC6 over 3*6bit
    let mut index = ((input >> 12) & 0x3F) as usize;
    let mut crc = ((input >> 6) & 0x3F) as usize;
    index = crc ^ TABLE_CRC6[index] as usize;
    crc = (input & 0x3F) as usize;
    index = crc ^ TABLE_CRC6[index] as usize;
    TABLE_CRC6[index]
}

/// Error type for MT6701
#[derive(Debug, Copy, Clone)]
pub struct Mt6701Error {
    pub crc_mismatch: bool,
    pub received_crc: u8,
    pub calculated_crc: u8,
}

pub struct Mt6701 {
    pub x: f32,
    pub y: f32,
    pub spi: Spi<'static, Blocking>,
    pub error: Option<Mt6701Error>,
}

impl Mt6701 {
    pub fn new(spi: Spi<'static, Blocking>) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            spi,
            error: None,
        }
    }

    /// Read raw angle and update filter; returns current angle in radians
    pub async fn read_angle(&mut self) -> f32 {
        let mut buf = [0u8; 3];
        // Perform SPI read
        self.spi.read(&mut buf).unwrap();

        let raw = u32::from(buf[0]) << 16 | u32::from(buf[1]) << 8 | u32::from(buf[2]);
        let angle_spi = raw >> 10;
        let received_crc = (raw & 0x3F) as u8;
        //defmt::info!("angle_spi: {}, crc: {}", angle_spi, received_crc);
        let calculated_crc = crc6_43_18bit(raw >> 6);

        if received_crc == calculated_crc {
            // Compute new vector
            let new_angle = (angle_spi as f32) * 2.0 * PI / 16384.0;
            let new_x = libm::cosf(new_angle);
            let new_y = libm::sinf(new_angle);
            self.x = ALPHA * new_x + (1.0 - ALPHA) * self.x;
            self.y = ALPHA * new_y + (1.0 - ALPHA) * self.y;
            self.error = None;
        } else {
            self.error = Some(Mt6701Error {
                crc_mismatch: true,
                received_crc,
                calculated_crc,
            });
        }
        // Compute angle
        let mut rad = -libm::atan2f(self.y, self.x);
        if rad < 0.0 {
            rad += 2.0 * PI;
        }
        rad
    }
}
