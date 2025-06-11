use core::f32::consts::{PI, TAU};

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use embassy_time::{Instant, Ticker};
use esp_hal::time::Rate;

use crate::{motor::mt6701::Mt6701, util::rate_to_duration};

static WATCH: Watch<CriticalSectionRawMutex, Angle, 1> = Watch::new();

const POLLING_RATE: Rate = Rate::from_hz(100);

#[derive(Clone, Copy)]
pub struct Angle {
    mechanical_angle: f32, //[rad]
    velocity: f32,         // [rad/s]
    angle: f32,            // [rad]
}

#[embassy_executor::task]
pub async fn motor_angle(mut mt6701: Mt6701) {
    let mut ticker = Ticker::every(rate_to_duration(POLLING_RATE));
    let mut full_rotations: i32 = 0;
    let mut last: Option<(f32, Instant)> = None;

    loop {
        let mechanical = mt6701.read_angle().await;
        let now = Instant::now();

        if let Some((prev_angle, _prev_time)) = last {
            let delta = mechanical - prev_angle;
            if delta > PI {
                full_rotations += 1;
            } else if delta < -PI {
                full_rotations -= 1;
            }
        }
        let continuous = full_rotations as f32 * TAU + mechanical;
        let velocity = if let Some((prev_angle, prev_time)) = last {
            let dt = 1_000_000.0 / now.duration_since(prev_time).as_micros() as f32;
            (continuous - (full_rotations as f32 * TAU + prev_angle)) / dt
        } else {
            0.0
        };
        WATCH.sender().send(Angle {
            mechanical_angle: mechanical,
            angle: continuous,
            velocity,
        });

        last = Some((mechanical, now));

        ticker.next().await;
    }
}
