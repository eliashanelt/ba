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
use smartknob::foc::Motor;
use smartknob::mt6701::Mt6701;
use smartknob::sensor::strain::Hx711;
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

    let driver = bldc::BldcDriver::init(uh, ul, wh, wl, vh, vl);
    let motor = bldc::BldcMotor::new(1, None, None, None, driver);

    spawner.spawn(motor_task(motor)).unwrap();

    spawner.spawn(led_ring(led_pin)).unwrap();
    spawner.spawn(strain_gauge(hx711)).unwrap();
    spawner.spawn(motor_angle(mt6701)).unwrap();

    let mut ticker = Ticker::every(Duration::from_secs(5));
    loop {
        info!("Hello World");
        ticker.next().await;
    }
}
