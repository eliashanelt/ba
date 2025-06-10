#![no_std]
#![no_main]

//! Modified on 2025-06-09
//! – Spins the BLDC at **0.25 Hz** (≈4 s per mechanical rev)
//! – LED ring shows the **absolute rotor angle** reported by the MT6701 *in real-time*,
//!   so if you stall the shaft by hand the LEDs freeze accordingly.
//!
//! The rest of the application remains identical to your original example,
//! except that everything now happens in one task – no `Channel` is needed.

use core::f32::consts::PI;
use core::ops::Mul;
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Ticker};
use embedded_hal_async::delay::DelayNs;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::mcpwm::operator::{PwmPin, PwmPinConfig};
use esp_hal::mcpwm::timer::PwmWorkingMode;
use esp_hal::mcpwm::{McPwm, PeripheralClockConfig};
use esp_hal::peripherals::MCPWM0;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{spi, Blocking};
use libm::{cosf, fabsf, sinf};
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
        .spawn(closed_loop_sine(uh, ul, wh, wl, vh, vl, mt6701))
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
        write_sk6805(&mut led_data, &leds);

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
    const P_PAIRS: usize = 4; // EM3215 → 4 pole pairs

    const FOC_PID_P: f32 = 1.0;
    const FOC_PID_I: f32 = 0.0;
    const FOC_PID_D: f32 = 0.148;
    const FOC_PID_OUTPUT_RAMP: f32 = 5000.0;
    const FOC_PID_LIMIT: f32 = 3.0;

    const FOC_VOLTAGE_LIMIT: f32 = 3.0;
    const FOC_LPF: f32 = 0.0075;
    //let mut foc = Foc::new();

    loop {
        let mech_angle = mt6701.read_angle().await;
        let elec_angle = (mech_angle * P_PAIRS as f32) % (2.0 * PI);
    }
}

// ======== Task: open-loop BLDC commutation (unchanged) ======================
//
#[embassy_executor::task]
async fn open_loop_sine(
    mut uh: PwmPin<'static, MCPWM0, 0, true>,
    mut ul: PwmPin<'static, MCPWM0, 0, false>,
    mut vh: PwmPin<'static, MCPWM0, 1, true>,
    mut vl: PwmPin<'static, MCPWM0, 1, false>,
    mut wh: PwmPin<'static, MCPWM0, 2, true>,
    mut wl: PwmPin<'static, MCPWM0, 2, false>,
) {
    use core::f32::consts::PI;
    use embassy_time::{Duration, Ticker};

    const PWM_MAX: u16 = 4095; // 12-bit resolution
    const P_PAIRS: usize = 4; // EM3215 → 4 pole pairs
    const F_MECH_HZ: f32 = 1.0; // 1 rev/s = 60 rpm
    const UPDATE_HZ: u32 = 2_000; // refresh 2 000 ×/s

    // --- pre-compute increments -----------------------------------------
    let electrical_hz = F_MECH_HZ * P_PAIRS as f32; // 4 Hz
    let d_theta = 2.0 * PI * electrical_hz / UPDATE_HZ as f32;

    // Offset phases for the three windings
    const A_PHASE: f32 = 0.0;
    const B_PHASE: f32 = -2.0 * PI / 3.0; // −120°
    const C_PHASE: f32 = 2.0 * PI / 3.0; // +120°

    // Amplitude (0.0–1.0). 0.25 is usually plenty for a gimbal motor.
    const MODULATION: f32 = 0.25;

    let mut theta = 0.0_f32;
    let mut ticker = Ticker::every(Duration::from_micros(1_000_000 / UPDATE_HZ as u64));

    loop {
        // --- compute three phase voltages (sinusoidal, centred 0.0–1.0) ---
        let va = sinf(theta + A_PHASE) * MODULATION + 0.5;
        let vb = sinf(theta + B_PHASE) * MODULATION + 0.5;
        let vc = sinf(theta + C_PHASE) * MODULATION + 0.5;

        // --- convert to timer counts --------------------------------------
        let da = (va * PWM_MAX as f32) as u16;
        let db = (vb * PWM_MAX as f32) as u16;
        let dc = (vc * PWM_MAX as f32) as u16;

        // --- write to half-bridges ----------------------------------------
        //   high-side = “positive”, low-side = complementary
        uh.set_duty_cycle(da);
        ul.set_duty_cycle(PWM_MAX - da);

        vh.set_duty_cycle(db);
        vl.set_duty_cycle(PWM_MAX - db);

        wh.set_duty_cycle(dc);
        wl.set_duty_cycle(PWM_MAX - dc);

        // --- advance electrical angle -------------------------------------
        theta += d_theta;
        if theta >= 2.0 * PI {
            theta -= 2.0 * PI;
        }

        ticker.next().await; // wait until next update slot
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
async fn closed_loop_sine(
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

#[embassy_executor::task]
async fn open_laoop_spin(
    mut uh: PwmPin<'static, MCPWM0, 0, true>,
    mut ul: PwmPin<'static, MCPWM0, 0, false>,
    mut vh: PwmPin<'static, MCPWM0, 1, true>,
    mut vl: PwmPin<'static, MCPWM0, 1, false>,
    mut wh: PwmPin<'static, MCPWM0, 2, true>,
    mut wl: PwmPin<'static, MCPWM0, 2, false>,
) {
    use embassy_time::Ticker;

    // Six-step (trapezoidal) table ─ unchanged
    const TABLE: [[u16; 6]; 6] = [
        [4095, 0, 0, 0, 0, 4095], // A+ C-
        [4095, 0, 0, 0, 4095, 0], // A+ B-
        [0, 0, 0, 4095, 4095, 0], // B+ C-
        [0, 4095, 0, 4095, 0, 0], // B+ A-
        [0, 4095, 4095, 0, 0, 0], // C+ A-
        [0, 0, 4095, 0, 0, 4095], // C+ B-
    ];

    let mut step = 0usize;
    let mut ticker = Ticker::every(Duration::from_micros(STEP_US as _));

    loop {
        let phase = TABLE[step];
        uh.set_duty_cycle(phase[0]);
        ul.set_duty_cycle(phase[1]);
        vh.set_duty_cycle(phase[2]);
        vl.set_duty_cycle(phase[3]);
        wh.set_duty_cycle(phase[4]);
        wl.set_duty_cycle(phase[5]);

        step = (step + 1) % 6;
        ticker.next().await;
    }
}
// ======== Utility functions / structs (unchanged) ===========================

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

pub enum MotionControlType {
    OpenAngleLoop,
    Torque,
}

pub struct SmartKnobConfig {}

pub struct MotorController {
    motor: BldcMotor,
}

pub struct BldcMotor {
    control_type: MotionControlType,
    pole_pairs: u32,
    zero_electric_angle: f32,
    sensor_direction: Direction,
    voltage_limit: f32,
}

impl BldcMotor {
    pub fn init_foc(&mut self) {}
    pub fn move_to(&mut self, value: f32) {}
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

    if moved_angle < rad_to_deg(30.0) || moved_angle > rad_to_deg(180.0) {
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

    motor.pole_pairs = measured_pole_pairs;
    motor.zero_electric_angle = avg_offset_angle + _3PI_2;
    motor.voltage_limit = FOC_VOLTAGE_LIMIT;
    motor.control_type = MotionControlType::Torque;
    Ok(())
}

enum CalibrationError {
    X,
}

fn rad_to_deg(rad: f32) -> f32 {
    (rad / 2.0 * PI) * 360.0
}
