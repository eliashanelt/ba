#![no_std]
#![no_main]

/*use core::f32::consts::{PI, TAU};
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Delay, Duration, Instant, Ticker, Timer};
use embedded_hal::delay::DelayNs;
use embedded_hal::pwm::SetDutyCycle;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig};
use esp_hal::mcpwm::operator::{PwmPin, PwmPinConfig};
use esp_hal::mcpwm::timer::PwmWorkingMode;
use esp_hal::mcpwm::{McPwm, PeripheralClockConfig};
use esp_hal::peripherals::MCPWM0;
use esp_hal::spi;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use libm::{atan2f, cosf, fabsf, floorf, sinf};
use panic_rtt_target as _;
use smartknob::config::SmartKnobConfig;
use smartknob::foc::Motor;
use smartknob::mt6701::Mt6701;
use smartknob::sensor::strain::HX711;
//use smartknob::sensor::strain::Hx711;
use smartknob::tasks::motor::{Press, MOTOR_CH};
use smartknob::tasks::motor_angle::motor_angle;
use smartknob::tasks::strain_gauge::strain_gauge;
use smartknob::tasks::{led_ring, motor_task};
use smartknob::{bldc, pid};

const PWM_FREQUENCY: Rate = Rate::from_khz(20);

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(esp_hal::clock::CpuClock::max());
    let p = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);
    let tg = TimerGroup::new(p.TIMG1);
    esp_hal_embassy::init(tg.timer0);

    let led_pin = Output::new(p.GPIO12, Level::High, OutputConfig::default());

    let clock_cfg = PeripheralClockConfig::with_frequency(Rate::from_mhz(40)).unwrap();
    let mut mcpwm = McPwm::new(p.MCPWM0, clock_cfg);

    mcpwm.operator0.set_timer(&mcpwm.timer0);
    mcpwm.operator1.set_timer(&mcpwm.timer1);
    mcpwm.operator2.set_timer(&mcpwm.timer2);

    let (uh, ul) = mcpwm.operator0.with_pins(
        p.GPIO8,
        PwmPinConfig::UP_ACTIVE_HIGH,
        p.GPIO16,
        PwmPinConfig::UP_ACTIVE_HIGH,
    );
    let (wh, wl) = mcpwm.operator2.with_pins(
        p.GPIO17,
        PwmPinConfig::UP_ACTIVE_HIGH,
        p.GPIO15,
        PwmPinConfig::UP_ACTIVE_HIGH,
    );
    let (vh, vl) = mcpwm.operator1.with_pins(
        p.GPIO18,
        PwmPinConfig::UP_ACTIVE_HIGH,
        p.GPIO7,
        PwmPinConfig::UP_ACTIVE_HIGH,
    );

    let timer_cfg = clock_cfg
        .timer_clock_with_frequency(1599, PwmWorkingMode::Increase, PWM_FREQUENCY)
        .unwrap();
    mcpwm.timer0.start(timer_cfg);
    mcpwm.timer1.start(timer_cfg);
    mcpwm.timer2.start(timer_cfg);

    let spi = spi::master::Spi::new(
        p.SPI2,
        spi::master::Config::default()
            .with_frequency(Rate::from_mhz(4))
            .with_mode(spi::Mode::_1),
    )
    .unwrap()
    .with_sck(p.GPIO13)
    .with_mosi(p.GPIO3)
    .with_miso(p.GPIO14)
    .with_cs(p.GPIO11);

    let mt6701 = Mt6701::new(spi);
    // --- HX711 strain gauge (unchanged) ------------------------------------
    let clk = Output::new(p.GPIO1, Level::Low, OutputConfig::default());
    let dout = Input::new(p.GPIO21, InputConfig::default());
    let hx711 = HX711::new(clk, dout, Delay);

    let driver = bldc::BldcDriver::init(uh, ul, vh, vl, wh, wl);
    let motor = bldc::BldcMotor::new(1, None, None, None, driver);

    //spawner.spawn(motor_task(motor)).unwrap();

    spawner.spawn(led_ring(led_pin)).unwrap();
    spawner.spawn(strain_gauge(hx711)).unwrap();
    spawner.spawn(motor_angle(mt6701)).unwrap();

    Timer::after(Duration::from_secs(5)).await;
    info!("calibration");
    MOTOR_CH
        .sender()
        .send(smartknob::tasks::motor::Command::Calibrate)
        .await;

    let mut ticker = Ticker::every(Duration::from_secs(5));
    loop {
        MOTOR_CH
            .sender()
            .send(smartknob::tasks::motor::Command::Haptic(Press::Short))
            .await;
        info!("Hello World");
        ticker.next().await;
    }
}
*/

// Calculate smooth complementary PWM with proper dead time
fn calculate_complementary_pwm(high_side_pwm: u8, dead_time_percent: u8) -> u8 {
    if high_side_pwm + dead_time_percent < 100 {
        100 - high_side_pwm - dead_time_percent
    } else {
        0
    }
}

// 6-step commutation for BLDC motors - provides more torque but causes vibrations
#[allow(dead_code)]
fn six_step_commutation(angle: f32, amplitude: f32) -> (u8, u8, u8) {
    let normalized_angle = angle % TAU;
    let max_pwm = (amplitude * 100.0) as u8;

    // Divide electrical cycle into 6 sectors (60° each)
    let sector = (normalized_angle / (TAU / 6.0)) as u8;

    match sector {
        0 => (max_pwm, 0, 0),       // U+, V-, W off
        1 => (max_pwm, max_pwm, 0), // U+, V+, W off
        2 => (0, max_pwm, 0),       // U off, V+, W-
        3 => (0, max_pwm, max_pwm), // U off, V+, W+
        4 => (0, 0, max_pwm),       // U-, V off, W+
        5 => (max_pwm, 0, max_pwm), // U+, V off, W+
        _ => (0, 0, 0),             // Default case
    }
}

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_hal::{pwm::SetDutyCycle, spi::SpiBus};
use esp_hal::{
    clock::CpuClock,
    gpio::{Io, Level, Output},
    mcpwm::{operator::PwmPinConfig, timer::PwmWorkingMode, McPwm, PeripheralClockConfig},
    spi,
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_hal_embassy::main;
use libm::sinf;
use panic_rtt_target as _;

// Constants
const PWM_FREQUENCY: Rate = Rate::from_khz(20); // 20kHz PWM frequency
const PWM_MAX: u16 = 1000; // Maximum PWM value
const PWM_AMPLITUDE: f32 = 0.85; // 85% amplitude for smooth operation
const MOTOR_SPEED: f32 = 3.0; // 3Hz electrical frequency - smooth rotation
const TAU: f32 = 2.0 * core::f32::consts::PI;
const DEAD_TIME_PERCENT: u8 = 3; // 3% dead time for safety
const MIN_PWM_PERCENT: u8 = 10; // Minimum PWM to maintain current flow
const SMOOTH_STARTUP: bool = true; // Enable smooth sinusoidal startup

#[main]
async fn main(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let p = esp_hal::init(config);
    esp_alloc::heap_allocator!(size: 72 * 1024);

    let tg = TimerGroup::new(p.TIMG1);
    esp_hal_embassy::init(tg.timer0);

    info!("Initializing BLDC motor control...");

    // Setup MCPWM
    let clock_cfg = PeripheralClockConfig::with_frequency(Rate::from_mhz(40)).unwrap();
    let mut mcpwm = McPwm::new(p.MCPWM0, clock_cfg);

    // Configure operators with their respective timers
    mcpwm.operator0.set_timer(&mcpwm.timer0);
    mcpwm.operator1.set_timer(&mcpwm.timer1);
    mcpwm.operator2.set_timer(&mcpwm.timer2);

    // Setup PWM pins for 3-phase motor control
    // Phase U (Red) - High and Low side
    let (mut uh, mut ul) = mcpwm.operator0.with_pins(
        p.GPIO8, // UH - Phase U High side
        PwmPinConfig::UP_ACTIVE_HIGH,
        p.GPIO16, // UL - Phase U Low side
        PwmPinConfig::UP_ACTIVE_HIGH,
    );

    // Phase V (Yellow) - High and Low side
    let (mut vh, mut vl) = mcpwm.operator1.with_pins(
        p.GPIO18, // VH - Phase V High side
        PwmPinConfig::UP_ACTIVE_HIGH,
        p.GPIO7, // VL - Phase V Low side
        PwmPinConfig::UP_ACTIVE_HIGH,
    );

    // Phase W (Blue) - High and Low side
    let (mut wh, mut wl) = mcpwm.operator2.with_pins(
        p.GPIO17, // WH - Phase W High side
        PwmPinConfig::UP_ACTIVE_HIGH,
        p.GPIO15, // WL - Phase W Low side
        PwmPinConfig::UP_ACTIVE_HIGH,
    );

    // Configure and start timers
    let timer_cfg = clock_cfg
        .timer_clock_with_frequency(PWM_MAX, PwmWorkingMode::Increase, PWM_FREQUENCY)
        .unwrap();

    mcpwm.timer0.start(timer_cfg);
    mcpwm.timer1.start(timer_cfg);
    mcpwm.timer2.start(timer_cfg);

    info!("Starting motor rotation with smooth sinusoidal control...");
    info!("Motor will start in 2 seconds...");
    Timer::after(Duration::from_millis(2000)).await;

    // Gradual startup sequence for smooth operation
    info!("Starting smooth rotation...");

    // Main loop - smooth sinusoidal 3-phase control
    let mut angle: f32 = 0.0;
    let mut current_amplitude = 0.1; // Start with low amplitude
    let target_amplitude = PWM_AMPLITUDE;
    let amplitude_ramp_rate = 0.001; // Gradual amplitude increase
    let angle_increment = TAU * MOTOR_SPEED / 1000.0;

    let mut spi = spi::slave::Spi::new(p.SPI2, spi::Mode::_0)
        .with_sck(p.GPIO45)
        .with_mosi(p.GPIO46)
        .with_miso(p.GPIO47)
        .with_cs(p.GPIO48);

    loop {
        continue;
        // Gradually ramp up amplitude for smooth startup
        if current_amplitude < target_amplitude {
            current_amplitude += amplitude_ramp_rate;
            if current_amplitude > target_amplitude {
                current_amplitude = target_amplitude;
            }
        }

        // Calculate smooth 3-phase sinusoidal signals (120° apart)
        let sin_u = sinf(angle);
        let sin_v = sinf(angle + TAU / 3.0); // 120° phase shift
        let sin_w = sinf(angle + 2.0 * TAU / 3.0); // 240° phase shift

        // Use Space Vector PWM for smoother control
        let (pwm_u_percent, pwm_v_percent, pwm_w_percent) =
            space_vector_pwm(angle, current_amplitude);

        // Alternative: Pure sinusoidal with offset for bipolar operation
        // This creates smoother operation than 6-step commutation
        let pwm_u_sin = ((sin_u * current_amplitude + 1.0) * 50.0)
            .clamp(MIN_PWM_PERCENT as f32, 100.0 - MIN_PWM_PERCENT as f32)
            as u8;
        let pwm_v_sin = ((sin_v * current_amplitude + 1.0) * 50.0)
            .clamp(MIN_PWM_PERCENT as f32, 100.0 - MIN_PWM_PERCENT as f32)
            as u8;
        let pwm_w_sin = ((sin_w * current_amplitude + 1.0) * 50.0)
            .clamp(MIN_PWM_PERCENT as f32, 100.0 - MIN_PWM_PERCENT as f32)
            as u8;

        // Use sinusoidal control for smooth operation
        let (pwm_u_final, pwm_v_final, pwm_w_final) = if SMOOTH_STARTUP {
            (pwm_u_sin, pwm_v_sin, pwm_w_sin)
        } else {
            (pwm_u_percent, pwm_v_percent, pwm_w_percent)
        };

        // Set PWM duty cycles for high-side switches
        let _ = uh.set_duty_cycle_percent(pwm_u_final);
        let _ = vh.set_duty_cycle_percent(pwm_v_final);
        let _ = wh.set_duty_cycle_percent(pwm_w_final);

        // Smooth complementary PWM for low-side switches
        // Use a more sophisticated approach for smoother operation
        let pwm_u_low = calculate_complementary_pwm(pwm_u_final, DEAD_TIME_PERCENT);
        let pwm_v_low = calculate_complementary_pwm(pwm_v_final, DEAD_TIME_PERCENT);
        let pwm_w_low = calculate_complementary_pwm(pwm_w_final, DEAD_TIME_PERCENT);

        let _ = ul.set_duty_cycle_percent(pwm_u_low);
        let _ = vl.set_duty_cycle_percent(pwm_v_low);
        let _ = wl.set_duty_cycle_percent(pwm_w_low);

        // Debug output every 1000 iterations (every 1 second)
        if (angle * 1000.0) as u32 % 6283 < 10 {
            // Approximately every 2π radians
            info!(
                "Angle: {}, Amp: {}, PWM%: U({}/{}), V({}/{}), W({}/{})",
                angle,
                current_amplitude,
                pwm_u_final,
                pwm_u_low,
                pwm_v_final,
                pwm_v_low,
                pwm_w_final,
                pwm_w_low
            );
        }

        // Increment angle for next iteration
        angle += angle_increment;
        if angle > TAU {
            angle -= TAU;
        }

        // Small delay for loop timing (1ms = 1kHz control frequency)
        Timer::after(Duration::from_millis(1)).await;
    }
}

// Alternative implementation with Space Vector PWM (more efficient and smoother)
fn space_vector_pwm(angle: f32, amplitude: f32) -> (u8, u8, u8) {
    // Normalize angle to [0, 2π]
    let normalized_angle = angle % TAU;

    // Determine sector (0-5, each sector is π/3 radians or 60°)
    let sector = (normalized_angle / (TAU / 6.0)) as u8;
    let sector_angle = normalized_angle % (TAU / 6.0);

    // Calculate time ratios for space vector modulation
    let sqrt3 = 1.732050808;
    let t1 = amplitude * sqrt3 * sinf(TAU / 3.0 - sector_angle);
    let t2 = amplitude * sqrt3 * sinf(sector_angle);
    let t0 = 1.0 - t1 - t2;

    // Ensure times are valid
    let t1 = t1.clamp(0.0, 1.0);
    let t2 = t2.clamp(0.0, 1.0);
    let t0 = t0.clamp(0.0, 1.0);

    // Calculate duty cycles based on sector with proper centering
    let (da, db, dc) = match sector {
        0 => (t1 + t2 + t0 / 2.0, t2 + t0 / 2.0, t0 / 2.0),
        1 => (t1 + t0 / 2.0, t1 + t2 + t0 / 2.0, t0 / 2.0),
        2 => (t0 / 2.0, t1 + t2 + t0 / 2.0, t2 + t0 / 2.0),
        3 => (t0 / 2.0, t1 + t0 / 2.0, t1 + t2 + t0 / 2.0),
        4 => (t2 + t0 / 2.0, t0 / 2.0, t1 + t2 + t0 / 2.0),
        5 => (t1 + t2 + t0 / 2.0, t0 / 2.0, t1 + t0 / 2.0),
        _ => (0.5, 0.5, 0.5), // Default case
    };

    // Convert to percentage (0-100) with proper clamping
    (
        (da * 100.0).clamp(5.0, 95.0) as u8,
        (db * 100.0).clamp(5.0, 95.0) as u8,
        (dc * 100.0).clamp(5.0, 95.0) as u8,
    )
}
