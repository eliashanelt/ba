use std::{
    fs::OpenOptions,
    io::{self, Read, Write},
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

const FREQ_HZ: f64 = 125_000_000.0; // 125 MHz
const BASE_ADDR: u64 = 0x4200_0000; // same physical address you used
const PAGE_SIZE: usize = 4096; // assumes 4 KiB pages on ARM
const ETHERNET_INTERFACE: &str = "eth0";

#[repr(C)]
struct RpRegs {
    count: u32,     // 0x00 – read-only cycle counter
    _reserved: u32, // 0x04
    config: u32,    // 0x08 – R/W   {phase_inc[31:5], log2_Ncycles[4:0]}
}

#[derive(serde::Deserialize)]
struct UiInput {
    value: f32,
}

#[derive(serde::Deserialize)]
struct VoltageInput {
    voltage: f32,
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
enum DelayMode {
    Period,
    Timestamp,
}

async fn handle_input(Json(payload): Json<UiInput>) -> impl IntoResponse {
    println!("Received slider value: {}", payload.value);
    // TODO: call into RP's C‐API via FFI, or manipulate GPIO/DAC here.
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn handle_voltage(Json(payload): Json<VoltageInput>) -> impl IntoResponse {
    println!("Received voltage value: {}V", payload.voltage);
    // TODO: call into RP's C‐API via FFI, or manipulate GPIO/DAC here.
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn handle_delay(Json(payload): Json<DelayInput>) -> impl IntoResponse {
    println!(
        "Received delay value: {} (mode: {:?})",
        payload.delay, payload.delay_mode
    );
    // TODO: call into RP's C‐API via FFI, or manipulate GPIO/DAC here.
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn handle_gain(Json(payload): Json<GainInput>) -> impl IntoResponse {
    println!("Received gain value: {}", payload.gain);
    // TODO: call into RP's C‐API via FFI, or manipulate GPIO/DAC here.
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn hello() -> impl IntoResponse {
    // TODO: call into RP's C‐API via FFI, or manipulate GPIO/DAC here.
    (StatusCode::OK, "Hello World!")
}

async fn handler_404() -> (StatusCode, Html<&'static str>) {
    (StatusCode::NOT_FOUND, Html("<h1>404 Not Found</h1>"))
}

#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel(100);
    let current_frequency = Arc::new(Mutex::new(1000.0f32));
    let voltage_lower = Arc::new(Mutex::new(-1.0f32));
    let voltage_upper = Arc::new(Mutex::new(1.0f32));
    let current_delay = Arc::new(Mutex::new(0.0f32));
    let current_delay_mode = Arc::new(Mutex::new(DelayMode::Period));
    let current_gain = Arc::new(Mutex::new(1.0f32));
    let freq_min = Arc::new(Mutex::new(100.0f32));
    let freq_max = Arc::new(Mutex::new(10000.0f32));

    let tx_monitor = tx.clone();
    let freq_monitor = current_frequency.clone();
    let voltage_lower_monitor = voltage_lower.clone();
    let voltage_upper_monitor = voltage_upper.clone();
    let delay_monitor = current_delay.clone();
    let delay_mode_monitor = current_delay_mode.clone();
    let gain_monitor = current_gain.clone();

    let app_state = AppState {
        tx: tx.clone(),
        current_frequency: current_frequency.clone(),
        voltage_lower: voltage_lower.clone(),
        voltage_upper: voltage_upper.clone(),
        current_delay: current_delay.clone(),
        current_delay_mode: current_delay_mode.clone(),
        current_gain: current_gain.clone(),
        freq_min: freq_min.clone(),
        freq_max: freq_max.clone(),
    };

    std::thread::spawn(move || {
        let (log2_ncycles, freq_in) = (10, 1000.0);

        let phase_inc: u32 = (2.147_482 * freq_in) as u32; // same constant factor
        let ncycles: u32 = 1 << log2_ncycles;
        // ─────────────────── open /dev/mem and mmap it ───────────────────
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/mem")
            .unwrap();

        let map: MmapMut = unsafe {
            MmapOptions::new()
                .offset(BASE_ADDR)
                .len(PAGE_SIZE)
                .map_mut(&file)
                .unwrap()
        };
        let regs = map.as_ptr() as *mut RpRegs;
        let cfg_word = (log2_ncycles & 0x1F) | (phase_inc << 5);
        unsafe { core::ptr::write_volatile(&mut (*regs).config, cfg_word) };

        loop {
            let count: u32 = unsafe { core::ptr::read_volatile(&(*regs).count) };
            let freq_est = (ncycles as f64 / count as f64) * FREQ_HZ;

            // Get current values for printing
            let lower = voltage_lower_monitor.lock().unwrap().clone();
            let upper = voltage_upper_monitor.lock().unwrap().clone();
            let delay = delay_monitor.lock().unwrap().clone();
            let delay_mode = delay_mode_monitor.lock().unwrap().clone();
            let gain = gain_monitor.lock().unwrap().clone();

            print!(
                "\rCounts: {count:5} | cycles/avg: {ncycles:5} | est. frequency: {freq_est:10.5} Hz | Voltage: {lower:+.3}V to {upper:+.3}V | Delay: {delay:.3} ({delay_mode:?}) | Gain: {gain:+.3}",
            );
            io::stdout().flush().ok();

            let _ = tx_monitor.send(WebSocketMessage::FrequencyUpdate {
                frequency: freq_est,
            });
            sleep(Duration::from_secs(3));
        }
    });

    let static_files =
        ServeDir::new("static").not_found_service(ServeFile::new("static/index.html"));
    // Build our app with a route for the API and the static assets
    let app = Router::new()
        .route("/api/input", post(handle_input))
        .route("/api/voltage", post(handle_voltage))
        .route("/api/delay", post(handle_delay))
        .route("/api/gain", post(handle_gain))
        .route("/api/hello", get(hello))
        .route("/ws", get(websocket_handler))
        .fallback_service(static_files) // Serve static assets like CSS, JS, images
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3030").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

const I14_MAX: i16 = 8191;
const I14_MIN: i16 = -8192;
fn is_ethernet_connected(interface: &str) -> io::Result<bool> {
    let path = format!("/sys/class/net/{interface}/carrier");
    let contents = std::fs::read_to_string(path)?;
    Ok(contents.trim() == "1")
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct RedpitayaStatus {
    connection_status: ConnectionStatus,
    fpga_status: FpgaStatus,
    frequency: f32, //[Hz]
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
    BinNotFound,
    ProgrammingError,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct ButtonSettings {}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct SystemStatus {}

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
    FrequencyUpdate { frequency: f64 },
    Error { message: String },
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<WebSocketMessage>,
    current_frequency: Arc<Mutex<f32>>,
    voltage_lower: Arc<Mutex<f32>>,
    voltage_upper: Arc<Mutex<f32>>,
    current_delay: Arc<Mutex<f32>>,
    current_delay_mode: Arc<Mutex<DelayMode>>,
    current_gain: Arc<Mutex<f32>>,
    freq_min: Arc<Mutex<f32>>,
    freq_max: Arc<Mutex<f32>>,
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
    let freq_clone = state.current_frequency.clone();
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
                            // TODO: call into RP's C‐API via FFI, or manipulate GPIO/DAC here.
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
                            // TODO: call into RP's C‐API via FFI, or manipulate GPIO/DAC here.
                            let _ =
                                tx_clone.send(WebSocketMessage::DelayUpdate { delay, delay_mode });
                        }
                        WebSocketMessage::SetGain { gain } => {
                            println!("Received gain value: {}", gain);
                            if let Ok(mut g) = gain_clone.lock() {
                                *g = gain;
                            }
                            // TODO: call into RP's C‐API via FFI, or manipulate GPIO/DAC here.
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
                            // TODO: Use this range to scale voltage output based on frequency
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
