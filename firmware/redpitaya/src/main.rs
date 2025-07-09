use std::{
    env,
    fs::OpenOptions,
    io::{self, Read, Write},
    thread::sleep,
    time::Duration,
};

use memmap2::{MmapMut, MmapOptions};
use std::f32::consts::PI;
use std::process::Command;

use embedded_hal::spi::SpiBus;
use embedded_hal::spi::SpiDevice;
use linux_embedded_hal::spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};

use rand_distr::{Distribution, Uniform};

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

    std::thread::spawn(|| {
        let (log2_ncycles, freq_in) = (10, 1000.0);

        let phase_inc: u32 = (2.147_482 * freq_in) as u32; // same constant factor
        let ncycles: u32 = 1 << log2_ncycles;
        // ─────────────────── open /dev/mem and mmap it ───────────────────
        let file = OpenOptions::new().read(true).write(true).open("/dev/mem")?;

        let map: MmapMut = unsafe {
            MmapOptions::new()
                .offset(BASE_ADDR)
                .len(PAGE_SIZE)
                .map_mut(&file)?
        };
        let regs = map.as_ptr() as *mut RpRegs;
        let cfg_word = (log2_ncycles & 0x1F) | (phase_inc << 5);
        unsafe { core::ptr::write_volatile(&mut (*regs).config, cfg_word) }; 
        loop {
            let count: u32 = unsafe { core::ptr::read_volatile(&(*regs).count) };
            let freq_est = (ncycles as f64 / count as f64) * FREQ_HZ;
            print!(
                "\rCounts: {:5} | cycles/avg: {:5} | est. frequency: {:10.5} Hz",
                count, ncycles, freq_est
            );
            io::stdout().flush().ok();
            sleep(Duration::from_secs(3));
        }
    });
    let (log2_ncycles, freq_in) = (10, 1000.0);

    let phase_inc: u32 = (2.147_482 * freq_in) as u32; // same constant factor
    let ncycles: u32 = 1 << log2_ncycles;
    // ─────────────────── open /dev/mem and mmap it ───────────────────
    let file = OpenOptions::new().read(true).write(true).open("/dev/mem")?;

    let map: MmapMut = unsafe {
        MmapOptions::new()
            .offset(BASE_ADDR)
            .len(PAGE_SIZE)
            .map_mut(&file)?
    };
    let regs = map.as_ptr() as *mut RpRegs;
    let cfg_word = (log2_ncycles & 0x1F) | (phase_inc << 5);
    unsafe { core::ptr::write_volatile(&mut (*regs).config, cfg_word) };

    /*let mut spi = Spidev::open("/dev/spidev1.0")?;
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(1_000_000) // 1 MHz
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options)?;*/

    println!("transfered");
    loop {
        /*let Ok(ethernet_connected) = is_ethernet_connected(ETHERNET_INTERFACE) else {
            continue;
        };

        let connection_status = if ethernet_connected {
            ConnectionStatus::EthernetConnected
        } else {
            ConnectionStatus::EthernetDisconnected
        };

        let wave_form = random_array();
        let msg = RedpitayaStatus {
            connection_status,
            fpga_status: FpgaStatus::BinNotFound,
            frequency: 69.0,
            wave_form,
        };
        let msg_bytes = postcard::to_stdvec(&msg)?;
        let receive_buffer = Vec::with_capacity(1024);
        let mut transfers = [
            &mut SpidevTransfer::write(&msg_bytes),
            &mut SpidevTransfer::read(&mut receive_buffer),
        ];
        spi.transfer(&mut SpidevTransfer::write(&msg_bytes))?;

        println!("{receive_buffer:?}");*/

        //print!("\x1B[2J\x1B[H");
        let count: u32 = unsafe { core::ptr::read_volatile(&(*regs).count) };
        let freq_est = (ncycles as f64 / count as f64) * FREQ_HZ;
        print!(
            "\rCounts: {:5} | cycles/avg: {:5} | est. frequency: {:10.5} Hz | count: {}",
            count, ncycles, freq_est, count
        );

        //let sine = sinus_period(I14_MIN, I14_MAX, 0);
        //print_graph(&sine, freq_est as f32);
        io::stdout().flush().ok();
        sleep(Duration::from_secs(3));
    }
}

fn random_array() -> [i16; 128] {
    let mut rng = rand::thread_rng();
    let uniform = Uniform::new(I14_MIN, I14_MAX).expect("try create uniform");
    std::array::from_fn(|_| uniform.sample(&mut rng))
}

const I14_MAX: i16 = 8191;
const I14_MIN: i16 = -8192;
fn is_ethernet_connected(interface: &str) -> io::Result<bool> {
    let path = format!("/sys/class/net/{}/carrier", interface);
    let contents = std::fs::read_to_string(path)?;
    Ok(contents.trim() == "1")
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct RedpitayaStatus {
    connection_status: ConnectionStatus,
    fpga_status: FpgaStatus,
    frequency: f32, //[Hz]
    #[serde(with = "BigArray")]
    wave_form: [u32; 128],
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum ConnectionStatus {
    EthernetConnected,
    EthernetDisconnected,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum FpgaStatus {
    BinNotFound,
    ProgrammingError,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct ButtonSettings {}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct SystemStatus {}

fn sinus_period(limit_min: i16, limit_max: i16, offset: i16) -> [i16; 128] {
    let mid = (I14_MAX as f32 + I14_MIN as f32) * 0.5;
    let amplitude = (I14_MAX as f32 - I14_MIN as f32) * 0.5;

    std::array::from_fn(|i| {
        // 0..127 sample index + offset, wrapped into [0..128)
        let phase = ((i as i16 + offset).rem_euclid(128) as f32) / 128.0;
        let angle = 2.0 * PI * phase;
        // full ±14-bit sample
        let raw = (mid + amplitude * angle.sin()).round() as i16;
        // clamp into your window
        raw.clamp(limit_min, limit_max)
    })
}

/// Print a text-mode graph of the data and display frequency & period
fn print_graph(data: &[i16; 128], frequency_hz: f32) {
    // Compute period in seconds
    let period_s = 1.0 / frequency_hz;
    // Choose appropriate time unit
    let (period_val, unit) = if period_s >= 1.0 {
        (period_s, "s")
    } else if period_s * 1e3 >= 1.0 {
        (period_s * 1e3, "ms")
    } else if period_s * 1e6 >= 1.0 {
        (period_s * 1e6, "us")
    } else {
        (period_s * 1e9, "ns")
    };

    // Header with frequency and period
    println!(
        "Frequency: {:.3} Hz   Period: {:.3} {}",
        frequency_hz, period_val, unit
    );

    // Determine data range
    let min_v = *data.iter().min().unwrap() as f32;
    let max_v = *data.iter().max().unwrap() as f32;
    let range = max_v - min_v;

    // Graph dimensions
    let height: usize = 20;
    let width = data.len();

    // Compute which rows correspond to max, zero, and min
    let row_of = |value: f32| -> usize {
        let norm = if range > 0.0 {
            (value - min_v) / range
        } else {
            0.5
        };
        let r = (norm * (height as f32 - 1.0)).round() as isize;
        // invert so 0 is bottom
        (height as isize - 1 - r) as usize
    };
    let row_max = row_of(max_v);
    let row_zero = if min_v <= 0.0 && max_v >= 0.0 {
        Some(row_of(0.0))
    } else {
        None
    };
    let row_min = row_of(min_v);

    // Prepare empty canvas
    let mut canvas = vec![vec![' '; width]; height];

    // Plot data
    for (x, &v) in data.iter().enumerate() {
        let y = row_of(v as f32);
        canvas[y][x] = '*';
    }

    // Overlay reference lines (but don't overwrite data points)
    for x in 0..width {
        // max
        if canvas[row_max][x] == ' ' {
            canvas[row_max][x] = '─';
        }
        // zero
        if let Some(rz) = row_zero {
            if canvas[rz][x] == ' ' {
                canvas[rz][x] = '─';
            }
        }
        // min
        if canvas[row_min][x] == ' ' {
            canvas[row_min][x] = '─';
        }
    }

    // Print each row with a y-axis label
    for row in 0..height {
        let value = max_v - (row as f32) * (range / (height as f32 - 1.0));
        print!("{:>7.1} | ", value);
        for &ch in &canvas[row] {
            print!("{}", ch);
        }
        println!();
    }

    // x-axis
    print!("{:>7} +", "");
    for _ in 0..width {
        print!("-");
    }
    println!();

    // tick marks every 16 samples
    print!("{:>7}   ", "");
    for i in 0..width {
        if i % 16 == 0 {
            print!("│");
        } else {
            print!(" ");
        }
    }
    println!();
    print!("{:>7}   ", "");
    for i in 0..width {
        if i % 16 == 0 {
            print!("{:^1}", i / 16);
        } else {
            print!(" ");
        }
    }
    println!();
}
