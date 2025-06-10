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

// ======== Application-wide constants ========================================

const LED_COUNT: usize = 72;
const CLK_HZ: u64 = 240_000_000;

const POLE_PAIRS: u32 = 4;
/// Mechanical period for 0.25 Hz = 4 000 000 µs
const REV_PERIOD_US: u32 = 4_000_000;
/// Commutation step every electrical 60 °
const STEP_US: u32 = REV_PERIOD_US / (6 * POLE_PAIRS); // = 83_333 µs

static ANGLE_CH: Channel<CriticalSectionRawMutex, f32, 1> = Channel::new();
static TARGET_CH: Channel<CriticalSectionRawMutex, f32, 1> = Channel::new();

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
    let mut led_data = Output::new(p.GPIO12, Level::High, OutputConfig::default());

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
        .timer_clock_with_frequency(1599, PwmWorkingMode::Increase, Rate::from_khz(20))
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
    let mut hx = Hx711::new(clk, dout);

    // ------------- Spawn the motor-drive task  ------------------------------
    spawner
        .spawn(motor_task(uh, ul, wh, wl, vh, vl, mt6701))
        .unwrap();

    // ­------------ Unified sampling + LED loop ------------
    let mut leds = [RGB8::default(); LED_COUNT];
    let mut ticker = Ticker::every(Duration::from_millis(4)); // 100 Hz
    let mut v = 0;
    loop {
        // ----- Acquire sensor data -----------------------------------------
        let angle = ANGLE_CH.receive().await;
        let strain = hx.read_raw().await;
        TARGET_CH.send((v as f32 / 100.0) * 2.0 * PI).await;
        v = (v + 1) % 100;
        // ----- Update LED ring ---------------------------------------------
        // Map 0-360° → 0-71 index.
        let idx =
            ((((angle / (2.0 * PI)) * LED_COUNT as f32 * 2.0) + 1.0) / 2.0) as usize % LED_COUNT;

        if strain > 400000 {
            leds.fill(RGB8 { r: 0, b: 5, g: 0 });
        } else {
            leds.fill(RGB8::default());
            leds[idx] = RGB8 {
                r: 255,
                g: 40,
                b: 0,
            };
        }
        //write_sk6805(&mut led_data, &leds);

        // ----- Logging ------------------------------------------------------

        ticker.next().await; // keep loop at 100 Hz
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

fn wrap_2pi(x: f32) -> f32 {
    let mut y = x % (2.0 * PI);
    if y < 0.0 {
        y += 2.0 * PI
    }
    y
}

#[embassy_executor::task]
async fn motor_task(
    mut uh: PwmPin<'static, MCPWM0, 0, true>,
    mut ul: PwmPin<'static, MCPWM0, 0, false>,
    mut vh: PwmPin<'static, MCPWM0, 1, true>,
    mut vl: PwmPin<'static, MCPWM0, 1, false>,
    mut wh: PwmPin<'static, MCPWM0, 2, true>,
    mut wl: PwmPin<'static, MCPWM0, 2, false>,
    mut mt6701: Mt6701,
) {
    // ---------- ① create a tiny “shadow” motor object --------------------
    let mut motor = BldcMotor {
        pwm: PhasePwm {
            uh,
            ul,
            vh,
            vl,
            wh,
            wl,
        },
        control_type: MotionControlType::OpenAngleLoop,
        pole_pairs: 1, // dummy, real value comes from calibration
        zero_electric_angle: 0.0,
        sensor_direction: Direction::CW,
        voltage_limit: 0.0,
        shaft_angle: 0.0,
        target: 0.0,
    };

    // ---------- ② run the calibration routine ----------------------------
    if let Err(e) = calibrate(&mut motor, &mut mt6701).await {
        error!("Calibration failed");
        panic!();
    }
    // ---------- ③ pull *measured* parameters back out --------------------
    let pole_pairs = motor.pole_pairs as f32;
    let zero_offset = motor.zero_electric_angle; // rad
    let dir_sign = f32::from(motor.sensor_direction); // +1 or -1

    info!(
        "PPairs={}, 0-elec offset={} rad, dir={}",
        pole_pairs, zero_offset, dir_sign
    );

    // ---------------------------------------------------------------------
    // From here on we revert to your original “closed_loop_sine” logic,
    // but replace the *hard-wired* constants with the values we just found.
    // ---------------------------------------------------------------------

    const PWM_MAX: u16 = 4095;
    const UPDATE_HZ: u32 = 2_000;
    const MODULATION: f32 = 0.7;
    const TORQUE_ADVANCE: f32 = core::f32::consts::FRAC_PI_2;
    const KP: f32 = 1.2;
    const KI: f32 = 25.0;

    let mut i_err = 0.0;
    let mut ticker = Ticker::every(Duration::from_micros(1_000_000 / UPDATE_HZ as u64));
    let mut target = 0.0;

    loop {
        if let Ok(t) = TARGET_CH.try_receive() {
            target = t;
        }

        // --- mechanical rotor angle from the encoder
        let mech = mt6701.read_angle().await * dir_sign;

        // --- electrical angle, incl. measured zero-offset
        let elec = mech * pole_pairs + zero_offset;
        let targ_e = target * pole_pairs;

        // --- PID on electrical angle error
        let err = wrap_2pi(targ_e - elec);
        i_err = (i_err + err / UPDATE_HZ as f32).clamp(-0.3, 0.3);
        let phase = elec + TORQUE_ADVANCE + KP * err + KI * i_err;

        // --- space-vector sine modulation exactly as before
        let (va, vb, vc) = (
            sinf(phase) * MODULATION + 0.5,
            sinf(phase - 2.0 * PI / 3.0) * MODULATION + 0.5,
            sinf(phase + 2.0 * PI / 3.0) * MODULATION + 0.5,
        );

        let (da, db, dc) = (
            (va * PWM_MAX as f32) as u16,
            (vb * PWM_MAX as f32) as u16,
            (vc * PWM_MAX as f32) as u16,
        );

        motor.pwm.set_phase_duties(da as f32, db as f32, dc as f32);
        // publish the (now calibrated) mechanical angle to the LED task
        ANGLE_CH.send(mech).await;

        ticker.next().await;
    }
}
#[embassy_executor::task]
async fn closed_loop_sinea(
    mut uh: PwmPin<'static, MCPWM0, 0, true>,
    mut ul: PwmPin<'static, MCPWM0, 0, false>,
    mut vh: PwmPin<'static, MCPWM0, 1, true>,
    mut vl: PwmPin<'static, MCPWM0, 1, false>,
    mut wh: PwmPin<'static, MCPWM0, 2, true>,
    mut wl: PwmPin<'static, MCPWM0, 2, false>,
    mut mt6701: Mt6701,
) {
    use core::f32::consts::PI;
    use embassy_time::{Duration, Ticker};

    const PWM_MAX: u16 = 4095;
    const P_PAIRS: f32 = 4.0;
    const UPDATE_HZ: u32 = 2_000;
    const MODULATION: f32 = 0.7;
    const TORQUE_ADVANCE: f32 = core::f32::consts::FRAC_PI_2;
    const KP: f32 = 1.2;
    const KI: f32 = 25.0;

    let mut i_err = 0.0;
    let mut ticker = Ticker::every(Duration::from_micros(1_000_000 / UPDATE_HZ as u64));
    let mut target = 0.0;
    loop {
        if let Ok(target_mech) = TARGET_CH.try_receive() {
            target = target_mech;
        }
        let mech = mt6701.read_angle().await; // 0–2π rad
        ANGLE_CH.send(mech).await;
        let elec = mech * P_PAIRS;
        let targ_e = target * P_PAIRS;

        let err = wrap_2pi(targ_e - elec);
        i_err = (i_err + err / UPDATE_HZ as f32).clamp(-0.3, 0.3);

        let phase = elec + TORQUE_ADVANCE + KP * err + KI * i_err;

        let (va, vb, vc) = (
            sinf(phase) * MODULATION + 0.5,
            sinf(phase - 2.0 * PI / 3.0) * MODULATION + 0.5,
            sinf(phase + 2.0 * PI / 3.0) * MODULATION + 0.5,
        );

        let (da, db, dc) = (
            (va * PWM_MAX as f32) as u16,
            (vb * PWM_MAX as f32) as u16,
            (vc * PWM_MAX as f32) as u16,
        );

        uh.set_duty_cycle(da);
        ul.set_duty_cycle(PWM_MAX - da);
        vh.set_duty_cycle(db);
        vl.set_duty_cycle(PWM_MAX - db);
        wh.set_duty_cycle(dc);
        wl.set_duty_cycle(PWM_MAX - dc);

        ticker.next().await;
    }
}

use embassy_time::Delay;
use embedded_hal::pwm::SetDutyCycle;

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

#[inline(always)]
fn send_bit(pin: &mut Output<'static>, bit_is_one: bool) {
    // ********  timing constants from the datasheet  ********
    const T0H_NS: u32 = ns_to_cycles(300); // 0-code high   (0.3 µs)
    const T0L_NS: u32 = ns_to_cycles(900); // 0-code low    (0.6 µs)
    const T1H_NS: u32 = ns_to_cycles(600); // 1-code high   (0.6 µs)
    const T1L_NS: u32 = ns_to_cycles(600); // 1-code low    (0.6 µs)

    // ********  generate the waveform  ********
    critical_section::with(|_| {
        pin.set_high();
        if bit_is_one {
            delay_cycles(T1H_NS); // “1” – stay high a bit longer
            pin.set_low();
            delay_cycles(T1L_NS);
        } else {
            delay_cycles(T0H_NS); // “0” – short high pulse
            pin.set_low();
            delay_cycles(T0L_NS);
        }
    });
}

#[inline(always)]
fn delay_cycles(cycles: u32) {
    esp_hal::xtensa_lx::timer::delay(cycles);
}

#[inline(always)]
const fn ns_to_cycles(ns: u32) -> u32 {
    (((CLK_HZ) * ns as u64 + 999_999_999) / 1_000_000_000) as u32
}

fn send_byte(pin: &mut Output<'static>, mut byte: u8) {
    for _ in 0..8 {
        send_bit(pin, (byte & 0x80) != 0);
        byte <<= 1;
    }
}

fn write_sk6805(pin: &mut Output<'static>, frame: &[RGB8]) {
    for rgb in frame {
        for &b in &[rgb.g, rgb.r, rgb.b] {
            send_byte(pin, b);
        }
    }
    Delay.delay_us(80);
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
    OpenAngleLoop,
    Torque,
}

pub struct SmartKnobConfig {}

pub struct MotorController {
    motor: BldcMotor,
}

pub struct PhasePwm {
    pub uh: PwmPin<'static, MCPWM0, 0, true>,
    pub ul: PwmPin<'static, MCPWM0, 0, false>,
    pub vh: PwmPin<'static, MCPWM0, 1, true>,
    pub vl: PwmPin<'static, MCPWM0, 1, false>,
    pub wh: PwmPin<'static, MCPWM0, 2, true>,
    pub wl: PwmPin<'static, MCPWM0, 2, false>,
}

impl PhasePwm {
    /// Write *three* centred-PWM duty cycles (`0.0 … 1.0`)
    /// and derive the complementary low-side outputs.
    fn set_phase_duties(&mut self, a: f32, b: f32, c: f32) {
        const PWM_MAX: u16 = 4095;
        let to_counts = |u: f32| -> u16 { (u.clamp(0.0, 1.0) * PWM_MAX as f32) as u16 };
        let (da, db, dc) = (to_counts(a), to_counts(b), to_counts(c));

        self.uh.set_duty_cycle(da);
        self.ul.set_duty_cycle(PWM_MAX - da);
        self.vh.set_duty_cycle(db);
        self.vl.set_duty_cycle(PWM_MAX - db);
        self.wh.set_duty_cycle(dc);
        self.wl.set_duty_cycle(PWM_MAX - dc);
    }
}
pub struct BldcMotor {
    pwm: PhasePwm,
    control_type: MotionControlType,
    pole_pairs: u32,
    zero_electric_angle: f32,
    sensor_direction: Direction,
    voltage_limit: f32,
    shaft_angle: f32,
    target: f32,
}

impl BldcMotor {
    pub fn init_foc(&mut self) {
        self.pwm.set_phase_duties(0.5, 0.5, 0.5);
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
        self.pwm.set_phase_duties(va, vb, vc);
    }

    /* ------------------------------------------------------------------ */
    /* 2.  `move_to` (ABSOLUTE) – wrapper expected by calibration         */
    /* ------------------------------------------------------------------ */
    pub fn move_by(&mut self, angle_elec: f32) {
        self.drive_elec(angle_elec);
    }

    /* ------------------------------------------------------------------ */
    /* 3.  `move_by` (RELATIVE) – optional helper for UI / haptics        */
    /* ------------------------------------------------------------------ */
    pub fn move_to(&mut self, delta_elec: f32) {
        // keep track of the current commanded angle in a field
        self.target += delta_elec;
        self.drive_elec(self.target);
    }
}

pub struct Foc<const PWM_RESOLUTION: u16> {
    flux_current_controller: pid::PIController,
    torque_current_controller: pid::PIController,
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
        motor.move_to(alpha);
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
