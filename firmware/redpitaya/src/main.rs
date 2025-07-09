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
async fn handle_input(Json(payload): Json<UiInput>) -> impl IntoResponse {
    println!("Received slider value: {}", payload.value);
    // TODO: call into RP’s C‐API via FFI, or manipulate GPIO/DAC here.
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn hello() -> impl IntoResponse {
    // TODO: call into RP’s C‐API via FFI, or manipulate GPIO/DAC here.
    (StatusCode::OK, "Hello World!")
}

async fn handler_404() -> (StatusCode, Html<&'static str>) {
    (StatusCode::NOT_FOUND, Html("<h1>404 Not Found</h1>"))
}
#[tokio::main]
async fn main() {
    /*let output = Command::new("fpgautil")
        .args(&["-b", "/root/passthrough.bit.bin"])
        .output() // returns io::Result<Output>
        .expect("Failed to launch fpgautil");

    // check if the command itself returned a zero exit code
    if output.status.success() {
        println!("✅ FPGA programmed successfully!");
    } else {
        // non-zero exit code → failure
        let code = output.status.code().map_or(-1, |c| c);
        eprintln!("❌ fpgautil failed (exit code {})", code);
        eprintln!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));
        // you can also return an Err here if you want to bail out
    }

    return Ok(());*/

    let (tx, _rx) = broadcast::channel(100);
    let current_frequency = Arc::new(Mutex::new(1000.0f32));

    let tx_monitor = tx.clone();
    let freq_monitor = current_frequency.clone();
    let app_state = AppState {
        tx: tx.clone(),
        current_frequency: current_frequency.clone(),
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
            print!(
                "\rCounts: {count:5} | cycles/avg: {ncycles:5} | est. frequency: {freq_est:10.5} Hz",
            );
            io::stdout().flush().ok();
            let _ = tx_monitor.send(WebSocketMessage::FrequencyUpdate {
                frequency: freq_est,
            });
            sleep(Duration::from_secs(3));
        }
    });

    /*let mut spi = Spidev::open("/dev/spidev1.0")?;
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(1_000_000) // 1 MHz
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options)?;*/

    let static_files =
        ServeDir::new("static").not_found_service(ServeFile::new("static/index.html"));
    // Build our app with a route for the API and the static assets
    let app = Router::new()
        .route("/api/input", post(handle_input))
        .route("/api/hello", get(hello))
        .route("/ws", get(websocket_handler))
        .fallback_service(static_files) // Serve static assets like CSS, JS, images
        .with_state(app_state);
    // Serve all other static files
    // Fallback for 404sk

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
    SliderUpdate { value: f32 },
    StatusUpdate { status: RedpitayaStatus },
    FrequencyUpdate { frequency: f64 },
    Hello,
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<WebSocketMessage>,
    current_frequency: Arc<Mutex<f32>>,
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

    // Spawn a task to handle incoming messages from this client
    let tx_clone = state.tx.clone();
    let freq_clone = state.current_frequency.clone();
    tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if let Ok(Message::Text(text)) = msg {
                if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                    match ws_msg {
                        WebSocketMessage::SliderUpdate { value } => {
                            println!("Received slider value: {value}");
                            // Update the current frequency
                            if let Ok(mut freq) = freq_clone.lock() {
                                *freq = value;
                            }

                            // TODO: call into RP's C‐API via FFI, or manipulate GPIO/DAC here.
                            // For now, just broadcast the update to all clients
                            let _ = tx_clone.send(WebSocketMessage::SliderUpdate { value });
                        }
                        WebSocketMessage::Hello => {
                            let _ = tx_clone.send(WebSocketMessage::Hello);
                        }
                        _ => {}
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
