#![no_std]
#![no_main]

use core::f32::consts::{PI, TAU};
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Delay, Duration, Instant, Ticker};
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
use smartknob::motor::mt6701::Mt6701;
use smartknob::pid;
use smartknob::sensor::strain::Hx711;
use smartknob::tasks::led_ring;
use smartknob::tasks::motor_angle::motor_angle;
use smartknob::tasks::strain_gauge::strain_gauge;

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
    let (wh, wl) = mcpwm.operator1.with_pins(
        p.GPIO17,
        PwmPinConfig::UP_ACTIVE_HIGH,
        p.GPIO7,
        PwmPinConfig::UP_ACTIVE_HIGH,
    );
    let (vh, vl) = mcpwm.operator2.with_pins(
        p.GPIO18,
        PwmPinConfig::UP_ACTIVE_HIGH,
        p.GPIO15,
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
    let hx711 = Hx711::new(clk, dout);

    /*spawner
    .spawn(motor_task(uh, ul, wh, wl, vh, vl, mt6701))
    .unwrap();*/

    spawner.spawn(led_ring(led_pin)).unwrap();
    spawner.spawn(strain_gauge(hx711)).unwrap();
    spawner.spawn(motor_angle(mt6701)).unwrap();

    let mut ticker = Ticker::every(Duration::from_secs(5));
    loop {
        info!("Hello World");
        ticker.next().await;
    }
}

async fn calibrate(motor: &mut BldcMotor, mt6701: &mut Mt6701) -> Result<(), CalibrationError> {
    motor.control_type = MotionControlType::AngleOpenLoop;
    motor.pole_pairs = 1;
    motor.zero_electric_angle = 0.0;
    motor.sensor_direction = Direction::CW;
    motor.init_foc();

    let mut alpha = 0.0;

    motor.voltage_limit = FOC_VOLTAGE_LIMIT;
    motor.move_to(alpha);

    for _ in 0..200 {
        motor.move_to(alpha);
        Delay.delay_ms(1);
    }
    let start_angle = mt6701.read_angle().await;

    while alpha < 3.0 * 2.0 * PI {
        motor.move_to(alpha);
        Delay.delay_ms(1);
        alpha += 0.01;
    }

    let end_angle = mt6701.read_angle().await;

    motor.voltage_limit = 0.0;
    motor.move_to(alpha);

    let moved_angle = fabsf(end_angle - start_angle);

    if moved_angle < deg_to_rad(30.0) || moved_angle > deg_to_rad(180.0) {
        error!(
            "Unexpected sensor change: start={}, end={}, moved={}",
            start_angle, end_angle, moved_angle
        );
        return Err(CalibrationError::X);
    }

    info!(
        "sensor change: start={}, end={}, moved={}",
        start_angle, end_angle, moved_angle
    );

    let direction = if end_angle > start_angle {
        Direction::CW
    } else {
        Direction::CCW
    };
    info!(
        "sensor measures positive change for positive motor rotaion: {}",
        direction == Direction::CW
    );
    motor.zero_electric_angle = 0.0;
    motor.sensor_direction = direction;
    motor.init_foc();

    let elec_revs = 20;
    info!("Going to measure {} electrical revolutions", elec_revs);
    motor.voltage_limit = FOC_VOLTAGE_LIMIT;
    motor.move_to(alpha);
    info!("Going to electrical zero");
    let dest = alpha + 2.0 * PI;
    while alpha < dest {
        motor.move_to(alpha);
        alpha += 0.03;
    }
    info!("Pause");
    Delay.delay_ms(1000);
    info!("Measuring");
    let start_angle = f32::from(motor.sensor_direction) * mt6701.read_angle().await;
    let dest = alpha + elec_revs as f32 * TAU;
    while alpha < dest {
        motor.move_to(alpha);
        alpha += 0.03;
    }
    info!("Pause");
    for _ in 0..1000 {
        motor.move_by(alpha);
        Delay.delay_ms(1);
    }
    let end_angle = f32::from(motor.sensor_direction) * mt6701.read_angle().await;
    motor.voltage_limit = 0.0;
    motor.move_to(alpha);

    if fabsf(motor.shaft_angle - motor.target) > PI / 180.0 {
        error!("motor did not reach target");
        return Err(CalibrationError::X);
    }
    info!("moved: {}", end_angle - start_angle);
    let elec_per_mech: f32 = elec_revs as f32 * TAU / (end_angle - start_angle);
    info!(
        "electrical angle / mechanical angle (i.e. pole pairs) = {}",
        elec_per_mech
    );
    if elec_per_mech < 3.0 || elec_per_mech > 12.0 {
        error!("Unexpected calculated pole pairs: {}", elec_per_mech);
        return Err(CalibrationError::X);
    }

    let measured_pole_pairs: u32 = (elec_per_mech as f32 + 0.5) as u32;
    info!("Pole pairs set to {}", measured_pole_pairs);

    Delay.delay_ms(1000);

    motor.voltage_limit = FOC_VOLTAGE_LIMIT;
    motor.move_to(alpha);
    let mut offset_x = 0.0;
    let mut offset_y = 0.0;
    let dest1 = (floorf(alpha / 2.0 * PI) + measured_pole_pairs as f32 / 2.0) * 2.0 * PI;
    let dest2 = floorf(alpha / 2.0 * PI) * 2.0 * PI;

    while alpha < dest1 {
        motor.move_to(alpha);
        Delay.delay_ms(200);
        let real_elec_angle = normalize_angle(alpha);
        let measured_elec_angle = normalize_angle(
            f32::from(motor.sensor_direction)
                * measured_pole_pairs as f32
                * mt6701.read_angle().await,
        );

        let offset_angle = measured_elec_angle - real_elec_angle;
        offset_x += cosf(offset_angle);
        offset_y += sinf(offset_angle);
        info!(
            "{},{},{}",
            rad_to_deg(real_elec_angle),
            rad_to_deg(measured_elec_angle),
            rad_to_deg(normalize_angle(offset_angle))
        );
        alpha += 0.4;
    }
    while alpha > dest2 {
        motor.move_to(alpha);
        Delay.delay_ms(200);
        let real_elec_angle = normalize_angle(alpha);
        let measured_elec_angle = normalize_angle(
            f32::from(motor.sensor_direction)
                * measured_pole_pairs as f32
                * mt6701.read_angle().await,
        );

        let offset_angle = measured_elec_angle - real_elec_angle;
        offset_x += cosf(offset_angle);
        offset_y += sinf(offset_angle);
        info!(
            "{},{},{}",
            rad_to_deg(real_elec_angle),
            rad_to_deg(measured_elec_angle),
            rad_to_deg(normalize_angle(offset_angle))
        );
        alpha -= 0.4;
    }
    motor.voltage_limit = 0.0;
    motor.move_to(alpha);

    let avg_offset_angle = atan2f(offset_y, offset_x);

    motor.pole_pairs = measured_pole_pairs;
    motor.zero_electric_angle = avg_offset_angle + 3.0 * PI / 2.0;
    motor.voltage_limit = FOC_VOLTAGE_LIMIT;
    motor.control_type = MotionControlType::Torque;

    info!(
        "calibration results: 0elec_angle={}, direction is cw={}",
        motor.zero_electric_angle,
        motor.sensor_direction == Direction::CW
    );

    Ok(())
}
