#![no_std]
#![no_main]

//! Modified on 2025-06-09
//! – Spins the BLDC at **0.25 Hz** (≈4 s per mechanical rev)
//! – LED ring shows the **absolute rotor angle** reported by the MT6701 *in real-time*,
//!   so if you stall the shaft by hand the LEDs freeze accordingly.
//!
//! The rest of the application remains identical to your original example,
//! except that everything now happens in one task – no `Channel` is needed.

use core::f32::consts::{PI, TAU};
use core::ops::Mul;
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Ticker};
use embedded_hal::delay::DelayNs;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::mcpwm::operator::{PwmPin, PwmPinConfig};
use esp_hal::mcpwm::timer::PwmWorkingMode;
use esp_hal::mcpwm::{McPwm, PeripheralClockConfig};
use esp_hal::peripherals::MCPWM0;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{spi, Blocking};
use libm::{atan2f, cosf, fabsf, floorf, sinf};
use panic_rtt_target as _;

use smartknob::motor::mt6701::Mt6701;
use smartknob::pid;
use smartknob::sensor::strain::Hx711;
use smartknob::tasks::led_ring;
use smartknob::tasks::strain_gauge::strain_gauge;
use tasks::led_ring;
// ======== Application-wide constants ========================================

const CLK_HZ: u64 = 240_000_000;

const POLE_PAIRS: u32 = 4;
/// Mechanical period for 0.25 Hz = 4 000 000 µs
const REV_PERIOD_US: u32 = 4_000_000;
/// Commutation step every electrical 60 °
const STEP_US: u32 = REV_PERIOD_US / (6 * POLE_PAIRS); // = 83_333 µs

static ANGLE_CH: Channel<CriticalSectionRawMutex, f32, 1> = Channel::new();
static TARGET_CH: Channel<CriticalSectionRawMutex, f32, 1> = Channel::new();

const PWM_FREQUENCY: Rate = Rate::from_khz(20);
// ======== Entry-point =======================================================

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(esp_hal::clock::CpuClock::max());
    let p = esp_hal::init(config);

    // --- Memory heap (unchanged) -------------------------------------------
    esp_alloc::heap_allocator!(size: 72 * 1024);

    // --- Timer for Embassy --------------------------------------------------
    let tg = TimerGroup::new(p.TIMG1);
    esp_hal_embassy::init(tg.timer0);

    // --- LED data pin -------------------------------------------------------
    let mut led_pin = Output::new(p.GPIO12, Level::High, OutputConfig::default());

    // --- MCPWM setup (unchanged, just shorter) -----------------------------
    let clock_cfg = PeripheralClockConfig::with_frequency(Rate::from_mhz(40)).unwrap();
    let mut mcpwm = McPwm::new(p.MCPWM0, clock_cfg);

    mcpwm.operator0.set_timer(&mcpwm.timer0);
    mcpwm.operator1.set_timer(&mcpwm.timer1);
    mcpwm.operator2.set_timer(&mcpwm.timer2);

    // Map BLDC phases: UH/UL, WH/WL, VH/VL ----------------------------------
    let (mut uh, mut ul) = mcpwm.operator0.with_pins(
        p.GPIO8,
        PwmPinConfig::UP_ACTIVE_HIGH,
        p.GPIO16,
        PwmPinConfig::UP_ACTIVE_HIGH,
    );
    let (mut wh, mut wl) = mcpwm.operator1.with_pins(
        p.GPIO17,
        PwmPinConfig::UP_ACTIVE_HIGH,
        p.GPIO7,
        PwmPinConfig::UP_ACTIVE_HIGH,
    );
    let (mut vh, mut vl) = mcpwm.operator2.with_pins(
        p.GPIO18,
        PwmPinConfig::UP_ACTIVE_HIGH,
        p.GPIO15,
        PwmPinConfig::UP_ACTIVE_HIGH,
    );

    // 20 kHz PWM, 12-bit resolution -----------------------------------------
    let timer_cfg = clock_cfg
        .timer_clock_with_frequency(1599, PwmWorkingMode::Increase, PWM_FREQUENCY)
        .unwrap();
    mcpwm.timer0.start(timer_cfg);
    mcpwm.timer1.start(timer_cfg);
    mcpwm.timer2.start(timer_cfg);

    // --- MT6701 absolute encoder -------------------------------------------
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

    let mut mt6701 = Mt6701 {
        spi,
        x: 0.0,
        y: 0.0,
        error: None,
    };

    // --- HX711 strain gauge (unchanged) ------------------------------------
    let clk = Output::new(p.GPIO1, Level::Low, OutputConfig::default());
    let dout = Input::new(p.GPIO21, InputConfig::default());
    let mut hx711 = Hx711::new(clk, dout);

    // ------------- Spawn the motor-drive task  ------------------------------
    /*spawner
        .spawn(motor_task(uh, ul, wh, wl, vh, vl, mt6701))
        .unwrap();*/

    // ­------------ Unified sampling + LED loop ------------
    spawner.spawn(led_ring(led_pin)).unwrap();
    spawner.spawn(strain_gauge(hx711)).unwrap();
    loop {
        info!("Hello World");
        ticker.next().await; // keep loop at 100 Hz
    }
}




pub enum Command {
    Calibrate,
    Config(SmartKnobConfig),
    Haptic(Press),
}

pub enum Press {
    Short,
    Long,
}

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

pub enum MotionControlType {
    Torque,
    Velocity,
    Angle,
    VelocityOpenLoop,
    AngleOpenLoop,
}

pub struct SmartKnobConfig {
    pub position_width_radians: f32,
    pub endstop_strength_unit: f32,
    pub snap_point: f32,
    pub detent_posiitons_count: u32,
    pub detent_positions: [u32; 5],
}

impl SmartKnobConfig {
    pub fn check(&self) -> Result<(), ConfigError> {
        // Check detent_strength_unit is not negative
        if self.detent_strength_unit < 0.0 {
            return Err(ConfigError::DetentStrengthNegative);
        }

        // Check endstop_strength_unit is not negative
        if self.endstop_strength_unit < 0.0 {
            return Err(ConfigError::EndstopStrengthNegative);
        }

        // Check snap_point is >= 0.5 for stability
        if self.snap_point < 0.5 {
            return Err(ConfigError::SnapPointTooSmall);
        }

        // Check detent_positions_count doesn't exceed array size
        if self.detent_positions_count > self.detent_positions.len() as u32 {
            return Err(ConfigError::DetentPositionsCountTooLarge);
        }

        // Check snap_point_bias is not negative
        if self.snap_point_bias < 0.0 {
            return Err(ConfigError::SnapPointBiasNegative);
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub enum ConfigError {
    DetentStrengthNegative,
    EndstopStrengthNegative,
    SnapPointTooSmall,
    DetentPositionsCountTooLarge,
    SnapPointBiasNegative,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::DetentStrengthNegative => {
                write!(f, "detent_strength_unit cannot be negative")
            }
            ConfigError::EndstopStrengthNegative => {
                write!(f, "endstop_strength_unit cannot be negative")
            }
            ConfigError::SnapPointTooSmall => {
                write!(f, "snap_point must be >= 0.5 for stability")
            }
            ConfigError::DetentPositionsCountTooLarge => {
                write!(f, "detent_positions_count is too large")
            }
            ConfigError::SnapPointBiasNegative => {
                write!(
                    f,
                    "snap_point_bias cannot be negative or there is risk of instability"
                )
            }
        }
    }
}

impl Default for SmartKnobConfig {
    fn default() -> Self {
        Self {
            position_width_radians: 60.0 * PI / 180.0,
            endstop_strength_unit: 0.0,
            snap_point: 0.5,
            detent_posiitons_count: 0,
            detent_positions: [0; 5],
        }
    }
}

#[derive(Clone, Copy)]
pub enum PhaseState {
    Off,
    On,
    High,
    Low,
}

pub struct MotorController {
    motor: BldcMotor,
}

pub struct BldcDriver {
    pub uh: PwmPin<'static, MCPWM0, 0, true>,
    pub ul: PwmPin<'static, MCPWM0, 0, false>,
    pub vh: PwmPin<'static, MCPWM0, 1, true>,
    pub vl: PwmPin<'static, MCPWM0, 1, false>,
    pub wh: PwmPin<'static, MCPWM0, 2, true>,
    pub wl: PwmPin<'static, MCPWM0, 2, false>,
    pub dead_zone: f32,
    pub pwm_frequency: Rate,
    pub phase_states: [PhaseState; 3],
}

impl BldcDriver {
    pub fn init(
        uh: PwmPin<'static, MCPWM0, 0, true>,
        ul: PwmPin<'static, MCPWM0, 0, false>,
        vh: PwmPin<'static, MCPWM0, 1, true>,
        vl: PwmPin<'static, MCPWM0, 1, false>,
        wh: PwmPin<'static, MCPWM0, 2, true>,
        wl: PwmPin<'static, MCPWM0, 2, false>,
    ) -> Self {
        Self {
            uh,
            ul,
            vh,
            vl,
            wh,
            wl,
            dead_zone: 0.02,
            pwm_frequency: PWM_FREQUENCY,
            phase_states: [PhaseState::Off; 3],
        }
    }
    pub fn enable(&mut self) {
        self.phase_states = [PhaseState::On; 3];
        self.set_phase_duties(0.0, 0.0, 0.0);
    }

    /// Write *three* centred-PWM duty cycles (`0.0 … 1.0`)
    /// and derive the complementary low-side outputs.
    /*fn set_phase_duties(&mut self, a: f32, b: f32, c: f32) {
        const PWM_MAX: u16 = 4095;
        let to_counts = |u: f32| -> u16 { (u.clamp(0.0, 1.0) * PWM_MAX as f32) as u16 };
        let (da, db, dc) = (to_counts(a), to_counts(b), to_counts(c));
        self.uh.set_duty_cycle(da);
        self.ul.set_duty_cycle(PWM_MAX - da);
        self.vh.set_duty_cycle(db);
        self.vl.set_duty_cycle(PWM_MAX - db);
        self.wh.set_duty_cycle(dc);
        self.wl.set_duty_cycle(PWM_MAX - dc);
    }*/

    pub fn set_phase_duties(&mut self, a: f32, b: f32, c: f32) {
        const PWM_MAX: u16 = 4095;
        let to_counts = |u: f32| -> u16 { (u.clamp(0.0, 1.0) * PWM_MAX as f32) as u16 };

        let duties = [a, b, c];

        // Handle each phase separately due to different const generic types
        for (i, &duty) in duties.iter().enumerate() {
            let (high_duty, low_duty) = match self.phase_states[i] {
                PhaseState::On => {
                    // Normal complementary PWM with deadtime
                    let high_duty = (duty - self.dead_zone).clamp(0.0, 1.0);
                    let low_duty = 1.0 - (duty + self.dead_zone).clamp(0.0, 1.0);
                    (to_counts(high_duty), to_counts(low_duty))
                }
                PhaseState::High => {
                    // Only high-side active
                    (to_counts(duty), 0)
                }
                PhaseState::Low => {
                    // Only low-side active
                    (0, to_counts(duty))
                }
                PhaseState::Off => {
                    // Both switches off
                    (0, 0)
                }
            };

            match i {
                0 => {
                    // Phase A (U)
                    self.uh.set_duty_cycle(high_duty);
                    self.ul.set_duty_cycle(low_duty);
                }
                1 => {
                    // Phase B (V)
                    self.vh.set_duty_cycle(high_duty);
                    self.vl.set_duty_cycle(low_duty);
                }
                2 => {
                    // Phase C (W)
                    self.wh.set_duty_cycle(high_duty);
                    self.wl.set_duty_cycle(low_duty);
                }
                _ => unreachable!(),
            }
        }
    }
}
pub struct BldcMotor {
    driver: BldcDriver,
    foc: Foc<4095>,
    control_type: MotionControlType,
    pole_pairs: u32,
    zero_electric_angle: f32,
    sensor_direction: Direction,
    voltage_limit: f32,
    shaft_angle: f32,
    target: f32,
}

#[derive(Clone, Copy)]
pub enum TorqueControlType {
    Voltage,
    DcCurrent,
    FocCurrent,
}

impl BldcMotor {
    pub fn init_foc(&mut self) {
        self.driver.set_phase_duties(0.5, 0.5, 0.5);
    }
    /*pub fn move_to(&mut self, elec_angle: f32) {
        const MODULATION: f32 = 0.25; // keep it gentle (≈25 % bus)
        let m = MODULATION.min(self.voltage_limit / FOC_VOLTAGE_LIMIT);

        let (va, vb, vc) = (
            sinf(elec_angle) * m + 0.5,
            sinf(elec_angle - 2.0 * PI / 3.0) * m + 0.5,
            sinf(elec_angle + 2.0 * PI / 3.0) * m + 0.5,
        );

        self.pwm.set_phase_duties(va, vb, vc);
    }*/

    /* ------------------------------------------------------------------ */
    /* 1.  The low-level driver – absolute electrical angle, drives HW   */
    /* ------------------------------------------------------------------ */
    pub fn drive_elec(&mut self, angle_elec: f32) {
        use libm::sinf;
        const MOD: f32 = 0.25;
        let m = MOD.min(self.voltage_limit / FOC_VOLTAGE_LIMIT);

        let (va, vb, vc) = (
            sinf(angle_elec) * m + 0.5,
            sinf(angle_elec - 2.0 * PI / 3.0) * m + 0.5,
            sinf(angle_elec + 2.0 * PI / 3.0) * m + 0.5,
        );
        self.driver.set_phase_duties(va, vb, vc);
    }

    /* ------------------------------------------------------------------ */
    /* 2.  `move_to` (ABSOLUTE) – wrapper expected by calibration         */
    /* ------------------------------------------------------------------ */
    pub fn move_to(&mut self, target: f32) {
        //self.drive_elec(angle_elec);
        match self.control_type {}
    }

    /* ------------------------------------------------------------------ */
    /* 3.  `move_by` (RELATIVE) – optional helper for UI / haptics        */
    /* ------------------------------------------------------------------ */
    pub fn move_by(&mut self, delta_elec: f32) {
        // keep track of the current commanded angle in a field
        self.target += delta_elec;
        self.drive_elec(self.target);
    }
}

pub struct Foc<const PWM_RESOLUTION: u16> {
    flux_current_controller: pid::PIController,
    torque_current_controller: pid::PIController,
    velocity_pid: pid::PIDController,
    motion_control_type: MotionControlType,
    torque_control_type: TorqueControlType,
}

const FRAC_1_SQRT_3: f32 = 0.57735027;
const SQRT_3: f32 = 1.7320508;

impl<const PWM_RESOLUTION: u16> Foc<PWM_RESOLUTION> {
    pub fn new(
        flux_current_controller: pid::PIController,
        torque_current_controller: pid::PIController,
    ) -> Self {
        Self {
            flux_current_controller,
            torque_current_controller,
        }
    }

    pub fn update(
        &mut self,
        currents: [f32; 2],
        angle: f32,
        desired_torque: f32,
        dt: f32,
    ) -> [u16; 3] {
        let sin_angle = sinf(angle);
        let cos_angle = cosf(angle);

        let orthogonal_current = clarke(ThreePhaseBalancedReferenceFrame {
            a: currents[0],
            b: currents[1],
        });

        let rotating_current = park(cos_angle, sin_angle, orthogonal_current);

        let v_d = self
            .flux_current_controller
            .update(rotating_current.d, 0.0, dt);
        let v_q = self
            .torque_current_controller
            .update(rotating_current.q, desired_torque, dt);

        // Inverse Park transform
        let orthogonal_voltage = inverse_park(
            cos_angle,
            sin_angle,
            RotatingReferenceFrame { d: v_d, q: v_q },
        );
        as_compare_value::<PWM_RESOLUTION>(orthogonal_voltage)
    }
}

pub fn clarke(inputs: ThreePhaseBalancedReferenceFrame) -> TwoPhaseReferenceFrame {
    TwoPhaseReferenceFrame {
        // Eq3
        alpha: inputs.a,
        // Eq4
        beta: FRAC_1_SQRT_3 * (inputs.a + 2.0 * inputs.b),
    }
}

pub fn park(
    cos_angle: f32,
    sin_angle: f32,
    inputs: TwoPhaseReferenceFrame,
) -> RotatingReferenceFrame {
    RotatingReferenceFrame {
        // Eq8
        d: cos_angle * inputs.alpha + sin_angle * inputs.beta,
        // Eq9
        q: cos_angle * inputs.beta - sin_angle * inputs.alpha,
    }
}

#[derive(Debug, Clone)]
pub struct ThreePhaseBalancedReferenceFrame {
    pub a: f32,
    pub b: f32,
}

#[derive(Debug, Clone)]
pub struct TwoPhaseReferenceFrame {
    pub alpha: f32,
    pub beta: f32,
}

#[derive(Debug, Clone)]
pub struct RotatingReferenceFrame {
    pub d: f32,
    pub q: f32,
}

pub fn inverse_park(
    cos_angle: f32,
    sin_angle: f32,
    inputs: RotatingReferenceFrame,
) -> TwoPhaseReferenceFrame {
    TwoPhaseReferenceFrame {
        // Eq10
        alpha: cos_angle * inputs.d - sin_angle * inputs.q,
        // Eq11
        beta: sin_angle * inputs.d + cos_angle * inputs.q,
    }
}

pub fn inverse_clarke(inputs: TwoPhaseReferenceFrame) -> ThreePhaseReferenceFrame {
    ThreePhaseReferenceFrame {
        // Eq5
        a: inputs.alpha,
        // Eq6
        b: (-inputs.alpha + SQRT_3 * inputs.beta) / 2.0,
        // Eq7
        c: (-inputs.alpha - SQRT_3 * inputs.beta) / 2.0,
    }
}
#[derive(Debug, Clone)]
pub struct ThreePhaseReferenceFrame {
    pub a: f32,
    pub b: f32,
    pub c: f32,
}

fn modulate(value: TwoPhaseReferenceFrame) -> [f32; 3] {
    // Convert alpha/beta to x/y/z
    let sqrt_3_alpha = SQRT_3 * value.alpha;
    let beta = value.beta;
    let x = beta;
    let y = (beta + sqrt_3_alpha) / 2.0;
    let z = (beta - sqrt_3_alpha) / 2.0;

    // Calculate which sector the value falls in
    let sector: u8 = match (
        x.is_sign_positive(),
        y.is_sign_positive(),
        z.is_sign_positive(),
    ) {
        (true, true, false) => 1,
        (_, true, true) => 2,
        (true, false, true) => 3,
        (false, false, true) => 4,
        (_, false, false) => 5,
        (false, true, false) => 6,
    };

    // Map a,b,c values to three phase
    let (ta, tb, tc);
    match sector {
        1 | 4 => {
            ta = x - z;
            tb = x + z;
            tc = -x + z;
        }
        2 | 5 => {
            ta = y - z;
            tb = y + z;
            tc = -y - z;
        }
        3 | 6 => {
            ta = y - x;
            tb = -y + x;
            tc = -y - x;
        }
        _ => unreachable!("invalid sector"),
    }

    [ta, tb, tc]
}

fn as_compare_value<const MAX: u16>(value: TwoPhaseReferenceFrame) -> [u16; 3] {
    modulate(value).map(|v| {
        // scale into [0 .. MAX+1] range
        let scaled = (v + 1.0) * ((MAX as f32) + 1.0) / 2.0;
        // add 0.5 for rounding-to-nearest, then clamp into [0 .. MAX]
        let rounded = (scaled + 0.5).clamp(0.0, MAX as f32);
        // truncate to u16
        rounded as u16
    })
}

const FOC_VOLTAGE_LIMIT: f32 = 5.0;

async fn calibrate(motor: &mut BldcMotor, mt6701: &mut Mt6701) -> Result<(), CalibrationError> {
    motor.control_type = MotionControlType::OpenAngleLoop;
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

enum CalibrationError {
    X,
}

fn rad_to_deg(rad: f32) -> f32 {
    (rad / 2.0 * PI) * 360.0
}

fn deg_to_rad(deg: f32) -> f32 {
    (deg / 360.0) * TAU
}

pub fn normalize_angle(mut angle: f32) -> f32 {
    const TAU: f32 = core::f32::consts::TAU; // 2π

    angle %= TAU;
    if angle < 0.0 {
        angle += TAU;
    }
    angle
}

static MOTOR_CH: Channel<CriticalSectionRawMutex, Command, 1> = Channel::new();

#[embassy_executor::task]
pub async fn motor_task2(mut motor: BldcMotor) {
    //motor.zero_electric_angle =
    let mut config = SmartKnobConfig::default();
    let mut current_position = 0;

    loop {
        //motor.foc.loop();
        if let Ok(command) = MOTOR_CH.try_receive() {
            match command {
                Command::Calibrate => {
                    motor.foc.velocity_pid = pid::PIDController::new(
                        FOC_PID_P,
                        FOC_PID_I,
                        FOC_PID_D,
                        Some(FOC_PID_OUTPUT_RAMP),
                        Some(FOC_PID_LIMIT),
                    );

                    //motor.calibrate();
                    //motor.init_foc();
                }
                Command::Config(new_config) => {
                    if let Err(e) = new_config.check() {
                        panic!("Invalid smartknob config: {e}");
                    }
                    let position_updated = false;

                    if new_config.position != config.position
                        || new_config.sub_position_unit != config.sub_position_unit
                        || new_config.position_nonce != config.position_nonce
                    {
                        info!("Applying position change");
                        current_position = new_config.position;
                        position_updated = true;
                    }

                    todo!()

                    if new_config.min_position <= new_config.max_position {
                        // Only check bounds if min/max indicate bounds are active (min >= max)
                        if current_position < new_config.min_position {
                            current_position = new_config.min_position;
                            info!("Adjusting position to min");
                        } else if current_position > new_config.max_position {
                            current_position = new_config.max_position;
                            info!("Adjusting position to max");
                        }
                    }
                    motor.foc.PID_velocity_pid.derivative.k_d = config.detent_positions_count > 0 ? 0 : CLAMP(raw, min(derivative_lower_strength, derivative_upper_strength), max(derivative_lower_strength, derivative_upper_strength));
                }
                Command::Haptic(press) => {
                    let (strength, foc_ticks) = match press {
                        Press::Short => (5.0, 3),
                        Press::Long => (20.0, 6),
                    }
                    //motor.move(strength);
                    for _ in 0..foc_ticks {
                        //motor.foc.loop()
                        Delay.delay_ms(1);
                    }
                    //motor.move(-strength);
                    for _ in 0..foc_ticks {
                        //motor.foc.loop()
                        Delay.delay_ms(1);
                    }
                    //motor.move(0);
                    //motor.foc.loop();
                },
            }
        }
    }
}
const FOC_PID_P: f32 = 4.0;
const FOC_PID_I: f32 = 0.0;
const FOC_PID_D: f32 = 0.04;
const FOC_PID_OUTPUT_RAMP: f32 = 10000.0;
const FOC_PID_LIMIT: f32 = 10.0;
