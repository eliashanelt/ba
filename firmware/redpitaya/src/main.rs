use std::{
    fs::OpenOptions,
    io::{self, Read, Write},
    process::Command,
    ptr::{read_volatile, write_volatile},
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
};

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
};
use memmap2::{MmapMut, MmapOptions};

use axum::response::Html;
use axum::{
    Json, Router,
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use tokio::sync::broadcast;
use tower_http::services::ServeFile;

use embedded_hal::spi::SpiBus;
use embedded_hal::spi::SpiDevice;
use linux_embedded_hal::spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};
use tower_http::services::ServeDir;

use futures_util::{
    sink::SinkExt,
    stream::{SplitSink, SplitStream, StreamExt},
};

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

const FPGA_BASE_ADDR: usize = 0x40000000; // Adjust based on your AXI address
const PAGE_SIZE: usize = 4096;
const FREQ_HZ: f64 = 125_000_000.0; // 125 MHz
const ETHERNET_INTERFACE: &str = "eth0";

// Memory abstraction layer functions
fn flash_fpga_with_fpgautil(bitstream: &str) -> std::io::Result<()> {
    let status = Command::new("fpgautil").arg("-b").arg(bitstream).status()?;

    if status.success() {
        println!("FPGA flashed successfully with '{bitstream}'");
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "fpgautil failed with exit code {}",
            status.code().unwrap_or(-1)
        )))
    }
}

fn sign_extend_14(val: u16) -> i16 {
    if val & 0x2000 != 0 {
        // Sign bit is set → negative number
        (val | 0xC000) as i16
    } else {
        val as i16
    }
}

fn f32_to_adc(x: f32) -> i16 {
    assert!(
        x.is_finite() && (-1.0..=1.0).contains(&x),
        "Input must be in [-1.0, 1.0], got {x}",
    );

    let scaled = (x * 8192.0).round();
    // Clamp to ensure we don't exceed i16::MAX
    scaled.clamp(-8192.0, 8191.0) as i16
}

fn adc_to_f32(x: i16) -> f32 {
    let val = if x > 0 {
        x as f32 / 8191.0
    } else if x < 0 {
        x as f32 / 8192.0
    } else {
        0.0
    };
    val.clamp(-1.0, 1.0)
}

fn combine_14bit_signed(a: i16, b: i16) -> u32 {
    assert!((-8192..=8191).contains(&a), "a out of 14-bit signed range");
    assert!((-8192..=8191).contains(&b), "b out of 14-bit signed range");

    let a_u = (a & 0x3FFF) as u16; // mask lower 14 bits
    let b_u = (b & 0x3FFF) as u16;

    ((a_u as u32) << 14) | (b_u as u32)
}

pub struct RedPitayaInterface {
    mmap: MmapMut,
    base_offset: usize,
}

impl RedPitayaInterface {
    pub fn new(base_address: usize) -> Result<Self, Box<dyn std::error::Error>> {
        // Open /dev/mem
        let file = OpenOptions::new().read(true).write(true).open("/dev/mem")?;

        // Calculate page-aligned offset
        let page_addr = base_address & !(PAGE_SIZE - 1);
        let page_offset = base_address - page_addr;

        // Memory map the region
        let mmap = unsafe {
            MmapOptions::new()
                .offset(page_addr as u64)
                .len(PAGE_SIZE)
                .map_mut(&file)?
        };

        Ok(RedPitayaInterface {
            mmap,
            base_offset: page_offset,
        })
    }

    // Write methods
    pub fn set_limits(&mut self, min: f32, max: f32) {
        assert!(
            (-1.0..=0.0).contains(&min),
            "min must be between -1.0 and 0.0",
        );
        assert!(
            (0.0..=1.0).contains(&max),
            "max must be between 0.0 and 1.0",
        );
        let min_i = f32_to_adc(min);
        let max_i = f32_to_adc(max);
        let limits_packed = combine_14bit_signed(max_i, min_i);
        unsafe {
            let addr = (self.mmap.as_mut_ptr() as *mut u32)
                .add(self.base_offset / 4)
                .add(3); // Offset 0x0C
            write_volatile(addr, limits_packed);
        }
    }

    pub fn set_gain(&mut self, gain: f32) {
        assert!(
            (-10.0..=10.0).contains(&gain),
            "gain must be between -10.0 and 10.0, got {gain}",
        );
        let gain_i = (gain * (2.0_f32).powi(10)).round() as i32;
        unsafe {
            let addr = (self.mmap.as_mut_ptr() as *mut i32)
                .add(self.base_offset / 4)
                .add(4); // Offset 0x10
            write_volatile(addr, gain_i);
        }
    }

    pub fn set_delay(&mut self, delay: u32) {
        unsafe {
            let addr = (self.mmap.as_mut_ptr() as *mut u32)
                .add(self.base_offset / 4)
                .add(5); // Offset 0x14
            write_volatile(addr, delay);
        }
    }

    pub fn set_fmin(&mut self, fmin: u32) {
        unsafe {
            let addr = (self.mmap.as_mut_ptr() as *mut u32)
                .add(self.base_offset / 4)
                .add(6); // Offset 0x14
            write_volatile(addr, fmin);
        }
    }
    pub fn set_fmax(&mut self, fmax: u32) {
        unsafe {
            let addr = (self.mmap.as_mut_ptr() as *mut u32)
                .add(self.base_offset / 4)
                .add(7); // Offset 0x14
            write_volatile(addr, fmax);
        }
    }

    // Read methods
    pub fn get_frequency(&self) -> f64 {
        let count = unsafe {
            let addr = (self.mmap.as_ptr() as *const u32)
                .add(self.base_offset / 4)
                .add(0); // Offset 0x00
            read_volatile(addr)
        };
        let log2_ncycles = 10;
        let ncycles: u32 = 1 << log2_ncycles;
        (ncycles as f64 / count as f64) * FREQ_HZ
    }

    pub fn get_vpp(&self) -> f32 {
        let vpp_u = unsafe {
            let addr = (self.mmap.as_ptr() as *const u32)
                .add(self.base_offset / 4)
                .add(1); // Offset 0x04
            read_volatile(addr)
        };
        let vpp_i = (vpp_u & 0x3FFF) as i16;
        adc_to_f32(vpp_i)
    }

    pub fn get_vp(&self) -> f32 {
        let vp_u = unsafe {
            let addr = (self.mmap.as_ptr() as *const u32)
                .add(self.base_offset / 4)
                .add(2); // Offset 0x08
            read_volatile(addr)
        };

        let vp_raw = (vp_u & 0x3FFF) as u16; // Extract 14-bit value
        let vp_signed = sign_extend_14(vp_raw); // Convert to i16
        adc_to_f32(vp_signed)
    }
}

// Thread-safe wrapper for RedPitayaInterface
struct SafeRedPitayaInterface {
    inner: Arc<Mutex<RedPitayaInterface>>,
}

impl SafeRedPitayaInterface {
    fn new(base_address: usize) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(SafeRedPitayaInterface {
            inner: Arc::new(Mutex::new(RedPitayaInterface::new(base_address)?)),
        })
    }

    fn clone_inner(&self) -> Arc<Mutex<RedPitayaInterface>> {
        self.inner.clone()
    }
}

// Web server types
#[derive(serde::Deserialize)]
struct UiInput {
    value: f32,
}

#[derive(serde::Deserialize)]
struct VoltageInput {
    lower: f32,
    upper: f32,
}

#[derive(serde::Deserialize)]
struct DelayInput {
    delay: f32,
    delay_mode: DelayMode,
}

#[derive(serde::Deserialize)]
struct GainInput {
    gain: f32,
}

#[derive(serde::Deserialize)]
struct FreqRangeInput {
    fmin: f32,
    fmax: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
enum DelayMode {
    Period,
    Timestamp,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct RedpitayaStatus {
    connection_status: ConnectionStatus,
    fpga_status: FpgaStatus,
    frequency: f32,
    vpp: f32,
    vp: f32,
    #[serde(with = "BigArray")]
    wave_form: [u32; 128],
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
enum ConnectionStatus {
    EthernetConnected,
    EthernetDisconnected,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
enum FpgaStatus {
    Ok,
    BinNotFound,
    ProgrammingError,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum WebSocketMessage {
    // Incoming messages
    SetVoltageLimits { lower_limit: f32, upper_limit: f32 },
    SetDelay { delay: f32, delay_mode: DelayMode },
    SetGain { gain: f32 },
    SetFrequencyRange { min_freq: f32, max_freq: f32 },
    Hello,

    // Outgoing messages
    VoltageUpdate { lower_limit: f32, upper_limit: f32 },
    DelayUpdate { delay: f32, delay_mode: DelayMode },
    GainUpdate { gain: f32 },
    FrequencyRangeUpdate { min_freq: f32, max_freq: f32 },
    StatusUpdate { status: RedpitayaStatus },
    FrequencyUpdate { frequency: f64, vpp: f32, vp: f32 },
    Error { message: String },
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<WebSocketMessage>,
    rp_interface: Arc<Mutex<RedPitayaInterface>>,
    current_frequency: Arc<Mutex<f32>>,
    voltage_lower: Arc<Mutex<f32>>,
    voltage_upper: Arc<Mutex<f32>>,
    current_delay: Arc<Mutex<f32>>,
    current_delay_mode: Arc<Mutex<DelayMode>>,
    current_gain: Arc<Mutex<f32>>,
    freq_min: Arc<Mutex<f32>>,
    freq_max: Arc<Mutex<f32>>,
}

async fn handle_voltage(
    State(state): State<AppState>,
    Json(payload): Json<VoltageInput>,
) -> impl IntoResponse {
    println!(
        "Received voltage limits: Lower: {}V, Upper: {}V",
        payload.lower, payload.upper
    );

    // Update state
    if let Ok(mut lower) = state.voltage_lower.lock() {
        *lower = payload.lower;
    }
    if let Ok(mut upper) = state.voltage_upper.lock() {
        *upper = payload.upper;
    }

    // Update hardware
    if let Ok(mut rp) = state.rp_interface.lock() {
        rp.set_limits(payload.lower, payload.upper);
    }

    // Broadcast update
    let _ = state.tx.send(WebSocketMessage::VoltageUpdate {
        lower_limit: payload.lower,
        upper_limit: payload.upper,
    });

    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn handle_delay(
    State(state): State<AppState>,
    Json(payload): Json<DelayInput>,
) -> impl IntoResponse {
    println!(
        "Received delay value: {} (mode: {:?})",
        payload.delay, payload.delay_mode
    );

    // Update state
    if let Ok(mut d) = state.current_delay.lock() {
        *d = payload.delay;
    }
    if let Ok(mut dm) = state.current_delay_mode.lock() {
        *dm = payload.delay_mode;
    }

    // Update hardware (convert delay based on mode if needed)
    if let Ok(mut rp) = state.rp_interface.lock() {
        // For now, just use the raw delay value
        rp.set_delay(payload.delay as u32);
    }

    // Broadcast update
    let _ = state.tx.send(WebSocketMessage::DelayUpdate {
        delay: payload.delay,
        delay_mode: payload.delay_mode,
    });

    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn handle_gain(
    State(state): State<AppState>,
    Json(payload): Json<GainInput>,
) -> impl IntoResponse {
    println!("Received gain value: {}", payload.gain);

    // Update state
    if let Ok(mut g) = state.current_gain.lock() {
        *g = payload.gain;
    }

    // Update hardware
    if let Ok(mut rp) = state.rp_interface.lock() {
        rp.set_gain(payload.gain);
    }

    // Broadcast update
    let _ = state
        .tx
        .send(WebSocketMessage::GainUpdate { gain: payload.gain });

    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}
async fn handle_freq_range(
    State(state): State<AppState>,
    Json(payload): Json<FreqRangeInput>,
) -> impl IntoResponse {
    println!(
        "Received freq values fmin: {}, fmax: {}",
        payload.fmin, payload.fmax
    );

    // Update state
    if let Ok(mut g) = state.freq_min.lock() {
        *g = payload.fmin;
    }
    if let Ok(mut g) = state.freq_max.lock() {
        *g = payload.fmax;
    }

    // Update hardware
    if let Ok(mut rp) = state.rp_interface.lock() {
        rp.set_fmin(payload.fmin as u32);
        rp.set_fmax(payload.fmax as u32);
    }

    // Broadcast update
    let _ = state.tx.send(WebSocketMessage::FrequencyRangeUpdate {
        min_freq: payload.fmin,
        max_freq: payload.fmax,
    });

    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn hello() -> impl IntoResponse {
    (StatusCode::OK, "Hello World!")
}

async fn handler_404() -> (StatusCode, Html<&'static str>) {
    (StatusCode::NOT_FOUND, Html("<h1>404 Not Found</h1>"))
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    // Send initial state to the new client
    {
        let lower = state.voltage_lower.lock().unwrap().clone();
        let upper = state.voltage_upper.lock().unwrap().clone();
        let delay = state.current_delay.lock().unwrap().clone();
        let delay_mode = state.current_delay_mode.lock().unwrap().clone();
        let gain = state.current_gain.lock().unwrap().clone();
        let min_freq = state.freq_min.lock().unwrap().clone();
        let max_freq = state.freq_max.lock().unwrap().clone();

        // Send initial values
        let messages = vec![
            WebSocketMessage::VoltageUpdate {
                lower_limit: lower,
                upper_limit: upper,
            },
            WebSocketMessage::DelayUpdate { delay, delay_mode },
            WebSocketMessage::GainUpdate { gain },
            WebSocketMessage::FrequencyRangeUpdate { min_freq, max_freq },
        ];

        for msg in messages {
            let json = serde_json::to_string(&msg).unwrap();
            let _ = sender.send(Message::Text(json.into())).await;
        }
    }

    // Spawn a task to handle incoming messages from this client
    let tx_clone = state.tx.clone();
    let rp_clone = state.rp_interface.clone();
    let voltage_lower_clone = state.voltage_lower.clone();
    let voltage_upper_clone = state.voltage_upper.clone();
    let delay_clone = state.current_delay.clone();
    let delay_mode_clone = state.current_delay_mode.clone();
    let gain_clone = state.current_gain.clone();
    let freq_min_clone = state.freq_min.clone();
    let freq_max_clone = state.freq_max.clone();

    tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if let Ok(Message::Text(text)) = msg {
                if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                    match ws_msg {
                        WebSocketMessage::SetVoltageLimits {
                            lower_limit,
                            upper_limit,
                        } => {
                            println!(
                                "Received voltage limits: Lower: {}V, Upper: {}V",
                                lower_limit, upper_limit
                            );
                            if let Ok(mut lower) = voltage_lower_clone.lock() {
                                *lower = lower_limit;
                            }
                            if let Ok(mut upper) = voltage_upper_clone.lock() {
                                *upper = upper_limit;
                            }
                            // Update hardware
                            if let Ok(mut rp) = rp_clone.lock() {
                                rp.set_limits(lower_limit, upper_limit);
                            }
                            let _ = tx_clone.send(WebSocketMessage::VoltageUpdate {
                                lower_limit,
                                upper_limit,
                            });
                        }
                        WebSocketMessage::SetDelay { delay, delay_mode } => {
                            println!("Received delay value: {} (mode: {:?})", delay, delay_mode);
                            if let Ok(mut d) = delay_clone.lock() {
                                *d = delay;
                            }
                            if let Ok(mut dm) = delay_mode_clone.lock() {
                                *dm = delay_mode;
                            }
                            // Update hardware
                            if let Ok(mut rp) = rp_clone.lock() {
                                rp.set_delay(delay as u32);
                            }
                            let _ =
                                tx_clone.send(WebSocketMessage::DelayUpdate { delay, delay_mode });
                        }
                        WebSocketMessage::SetGain { gain } => {
                            println!("Received gain value: {}", gain);
                            if let Ok(mut g) = gain_clone.lock() {
                                *g = gain;
                            }
                            // Update hardware
                            if let Ok(mut rp) = rp_clone.lock() {
                                rp.set_gain(gain);
                            }
                            let _ = tx_clone.send(WebSocketMessage::GainUpdate { gain });
                        }
                        WebSocketMessage::SetFrequencyRange { min_freq, max_freq } => {
                            println!("Received frequency range: {} - {} Hz", min_freq, max_freq);
                            if let Ok(mut min) = freq_min_clone.lock() {
                                *min = min_freq;
                            }
                            if let Ok(mut max) = freq_max_clone.lock() {
                                *max = max_freq;
                            }
                            if let Ok(mut rp) = rp_clone.lock() {
                                rp.set_fmin(min_freq as u32);
                                rp.set_fmax(max_freq as u32);
                            }
                            let _ = tx_clone.send(WebSocketMessage::FrequencyRangeUpdate {
                                min_freq,
                                max_freq,
                            });
                        }
                        WebSocketMessage::Hello => {
                            let _ = tx_clone.send(WebSocketMessage::Hello);
                        }
                        _ => {
                            // Ignore outgoing message types
                        }
                    }
                }
            }
        }
    });

    // Handle outgoing messages to this client
    while let Ok(msg) = rx.recv().await {
        let json = serde_json::to_string(&msg).unwrap();
        if sender.send(Message::Text(json.into())).await.is_err() {
            break;
        }
    }
}

fn is_ethernet_connected(interface: &str) -> io::Result<bool> {
    let path = format!("/sys/class/net/{interface}/carrier");
    let contents = std::fs::read_to_string(path)?;
    Ok(contents.trim() == "1")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Flash FPGA if needed
    if let Err(e) = flash_fpga_with_fpgautil("system_wrapper.bit.bin") {
        eprintln!("Warning: Failed to flash FPGA: {e}");
    }

    // Create RedPitaya interface
    let rp_interface = Arc::new(Mutex::new(RedPitayaInterface::new(FPGA_BASE_ADDR)?));

    // Initialize broadcast channel and shared state
    let (tx, _rx) = broadcast::channel(100);
    let current_frequency = Arc::new(Mutex::new(1000.0f32));
    let voltage_lower = Arc::new(Mutex::new(-1.0f32));
    let voltage_upper = Arc::new(Mutex::new(1.0f32));
    let current_delay = Arc::new(Mutex::new(0.0f32));
    let current_delay_mode = Arc::new(Mutex::new(DelayMode::Period));
    let current_gain = Arc::new(Mutex::new(1.0f32));
    let freq_min = Arc::new(Mutex::new(10_000.0f32));
    let freq_max = Arc::new(Mutex::new(1_000_000.0f32));

    // Set initial hardware values
    {
        let mut rp = rp_interface.lock().unwrap();
        rp.set_limits(-1.0, 1.0);
        rp.set_gain(1.0);
        rp.set_delay(0);
        rp.set_fmin(10_000);
        rp.set_fmax(1_000_000);
    }

    // Clone for monitor thread
    let tx_monitor = tx.clone();
    let rp_monitor = rp_interface.clone();
    let freq_monitor = current_frequency.clone();
    let voltage_lower_monitor = voltage_lower.clone();
    let voltage_upper_monitor = voltage_upper.clone();
    let delay_monitor = current_delay.clone();
    let delay_mode_monitor = current_delay_mode.clone();
    let gain_monitor = current_gain.clone();

    let app_state = AppState {
        tx: tx.clone(),
        rp_interface: rp_interface.clone(),
        current_frequency: current_frequency.clone(),
        voltage_lower: voltage_lower.clone(),
        voltage_upper: voltage_upper.clone(),
        current_delay: current_delay.clone(),
        current_delay_mode: current_delay_mode.clone(),
        current_gain: current_gain.clone(),
        freq_min: freq_min.clone(),
        freq_max: freq_max.clone(),
    };

    // Spawn monitoring thread
    std::thread::spawn(move || {
        loop {
            // Read values from hardware
            let (freq_est, vpp, vp) = {
                let rp = rp_monitor.lock().unwrap();
                (rp.get_frequency(), rp.get_vpp(), rp.get_vp())
            };

            // Update frequency state
            if let Ok(mut freq) = freq_monitor.lock() {
                *freq = freq_est as f32;
            }

            // Get current values for printing
            let lower = voltage_lower_monitor.lock().unwrap().clone();
            let upper = voltage_upper_monitor.lock().unwrap().clone();
            let delay = delay_monitor.lock().unwrap().clone();
            let delay_mode = delay_mode_monitor.lock().unwrap().clone();
            let gain = gain_monitor.lock().unwrap().clone();

            /*print!(
                "\rFreq: {freq_est:10.5} Hz | Vpp: {vpp:+.3} | Vp: {vp:+.3} | Voltage: {lower:+.3}V to {upper:+.3}V | Delay: {delay:.3} ({delay_mode:?}) | Gain: {gain:+.3}",
            );*/
            io::stdout().flush().ok();

            let _ = tx_monitor.send(WebSocketMessage::FrequencyUpdate {
                frequency: freq_est,
                vpp,
                vp,
            });

            // Check ethernet status and create full status update periodically
            if let Ok(eth_connected) = is_ethernet_connected(ETHERNET_INTERFACE) {
                let status = RedpitayaStatus {
                    connection_status: if eth_connected {
                        ConnectionStatus::EthernetConnected
                    } else {
                        ConnectionStatus::EthernetDisconnected
                    },
                    fpga_status: FpgaStatus::Ok,
                    frequency: freq_est as f32,
                    vpp,
                    vp,
                    wave_form: [0; 128], // TODO: Implement waveform capture
                };
                let _ = tx_monitor.send(WebSocketMessage::StatusUpdate { status });
            }

            sleep(Duration::from_millis(500);
        }
    });

    // Set up web server
    let static_files =
        ServeDir::new("static").not_found_service(ServeFile::new("static/index.html"));

    let app = Router::new()
        .route("/api/voltage", post(handle_voltage))
        .route("/api/delay", post(handle_delay))
        .route("/api/gain", post(handle_gain))
        .route("/api/freq-range", post(handle_freq_range))
        .route("/api/hello", get(hello))
        .route("/ws", get(websocket_handler))
        .fallback_service(static_files)
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3030").await.unwrap();
    println!("Server running on http://0.0.0.0:3030");
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
