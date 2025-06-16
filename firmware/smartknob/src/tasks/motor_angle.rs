use crate::{mt6701::Mt6701, util::rate_to_duration};
use core::f32::consts::{PI, TAU};
use embassy_futures::select::{select, Either};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal, watch::Watch};
use embassy_time::{Instant, Ticker};
use esp_hal::time::Rate;

static WATCH: Watch<CriticalSectionRawMutex, RotorState, 1> = Watch::new();
static TRIGGER: Signal<CriticalSectionRawMutex, ()> = Signal::new();
const POLLING_RATE: Rate = Rate::from_hz(100);

pub struct AngleSensor {
    pub needs_zero_search: bool,
}

impl AngleSensor {
    pub fn new() -> Self {
        Self {
            needs_zero_search: true,
        }
    }

    pub async fn electrical_angle(&self) -> f32 {
        TRIGGER.signal(());
        let mut receiver = WATCH.receiver().unwrap();
        receiver.changed().await;
        let new_state = receiver.get().await;

        new_state.angle
    }

    pub async fn mechanical_angle(&self) -> f32 {
        TRIGGER.signal(());

        let mut receiver = WATCH.receiver().unwrap();
        receiver.changed().await;
        let state = receiver.get().await;

        state.mechanical_angle
    }

    pub async fn velocity(&self) -> f32 {
        TRIGGER.signal(());

        let mut receiver = WATCH.receiver().unwrap();
        receiver.changed().await;
        let state = receiver.get().await;

        state.velocity
    }
}

#[derive(Clone, Copy)]
pub struct RotorState {
    pub mechanical_angle: f32, // [rad]
    pub velocity: f32,         // [rad/s]
    pub angle: f32,            // [rad]
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
                full_rotations -= 1; // Fixed: should be -= when angle wraps forward
            } else if delta < -PI {
                full_rotations += 1; // Fixed: should be += when angle wraps backward
            }
        }

        let continuous = full_rotations as f32 * TAU + mechanical;
        let velocity = if let Some((prev_angle, prev_time)) = last {
            let dt = 1_000_000.0 / now.duration_since(prev_time).as_micros() as f32;
            let prev_continuous = full_rotations as f32 * TAU + prev_angle;
            (continuous - prev_continuous) / dt
        } else {
            0.0
        };

        last = Some((mechanical, now));

        WATCH.sender().send(RotorState {
            mechanical_angle: mechanical,
            angle: continuous,
            velocity,
        });

        // Wait for either the next tick or a trigger
        match select(ticker.next(), TRIGGER.wait()).await {
            Either::First(_) => {
                // Regular polling tick
            }
            Either::Second(_) => {
                // Triggered measurement - loop again immediately
                TRIGGER.reset();
            }
        }
    }
}
