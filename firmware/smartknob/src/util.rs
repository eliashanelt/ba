use embassy_time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    CW,
    CCW,
}

impl From<Direction> for f32 {
    fn from(val: Direction) -> Self {
        match val {
            Direction::CW => 1.0,
            Direction::CCW => -1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MovingAverage<const SIZE: usize> {
    buf: [f32; SIZE],
    sum: f32,
    idx: usize,
    count: usize,
}

impl<const SIZE: usize> MovingAverage<SIZE> {
    /// Create a new empty mover over a zero‐initialized buffer.
    pub fn new() -> Self {
        Self {
            buf: [0.0; SIZE],
            sum: 0.0,
            idx: 0,
            count: 0,
        }
    }

    /// Push a new value, return the current average.
    pub fn add(&mut self, value: f32) -> f32 {
        // subtract the oldest, add the new
        let old = self.buf[self.idx];
        self.sum += value;
        self.sum -= old;
        self.buf[self.idx] = value;

        // advance the circular index
        self.idx = (self.idx + 1) % SIZE;
        if self.count < SIZE {
            self.count += 1;
        }

        self.sum / self.count as f32
    }
}

pub fn lerp(val: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    ((val - in_min) / (in_max - in_min)) * (out_max - out_min) + out_min
}

pub fn normalize_angle(mut angle: f32) -> f32 {
    const TAU: f32 = core::f32::consts::TAU;

    angle %= TAU;
    if angle < 0.0 {
        angle += TAU;
    }
    angle
}

pub fn millis() -> u32 {
    Instant::now().as_millis() as u32
}
