use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
    thread::sleep,
    time::Duration,
};

use memmap2::{MmapMut, MmapOptions};
use std::process::Command;

use embedded_hal::spi::SpiBus;
use embedded_hal::spi::SpiDevice;
use linux_embedded_hal::spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};
use serde::{Deserialize, Serialize};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("fpgautil -b /root/fpga.bit.bin")
        .output()
        .expect("failed to execute command");

    println!("{}\n{}\nerror: {}", output.stdout, output.stderr, output.exit_ok());
    return Ok(())

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

    let mut spi = Spidev::open("/dev/spidev1.0")?;
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(1_000_000) // 1 MHz
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options)?;

    println!("transfered");
    loop {
        let Ok(ethernet_connected) = is_ethernet_connected(ETHERNET_INTERFACE) else {
            continue;
        };

        let connection_status = if ethernet_connected {
            ConnectionStatus::EthernetConnected
        } else {
            ConnectionStatus::EthernetDisconnected
        };
        let msg = RedpitayaStatus {
            connection_status,
            fpga_status: todo!(),
            frequency: todo!(),
        };
        let msg_bytes = postcard::to_stdvec(&msg)?;
        spi.transfer(&mut SpidevTransfer::write(&msg_bytes))?;

        let count: u32 = unsafe { core::ptr::read_volatile(&(*regs).count) };
        let freq_est = (ncycles as f64 / count as f64) * FREQ_HZ;
        print!(
            "\rCounts: {:5} | cycles/avg: {:5} | est. frequency: {:10.5} Hz",
            count, ncycles, freq_est
        );
        io::stdout().flush().ok();
        sleep(Duration::from_secs(3));
    }
}

fn random_array() -> [f32; 128] {
    let mut rng = thread_rng();
    let uniform = Uniform::new(-1.0f32, 1.0f32);
    std::array::from_fn(|_| uniform.sample(&mut rng))
}

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
