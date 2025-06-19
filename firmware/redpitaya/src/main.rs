use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
    thread::sleep,
    time::Duration,
};

use embedded_hal::i2c::I2c;
use linux_embedded_hal::I2cdev;
use memmap2::{MmapMut, MmapOptions};
use std::process::Command;

const FREQ_HZ: f64 = 125_000_000.0; // 125 MHz
const BASE_ADDR: u64 = 0x4200_0000; // same physical address you used
const PAGE_SIZE: usize = 4096; // assumes 4 KiB pages on ARM

#[repr(C)]
struct RpRegs {
    count: u32,     // 0x00 – read-only cycle counter
    _reserved: u32, // 0x04
    config: u32,    // 0x08 – R/W   {phase_inc[31:5], log2_Ncycles[4:0]}
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let (log2_ncycles, freq_in) = match args.as_slice() {
        [_, l2, f] => (
            l2.parse::<u32>().unwrap_or(1),
            f.parse::<f64>().unwrap_or(1.0),
        ),
        _ => (1, 1.0),
    };

    let output = Command::new("fpgautil -b /root/fpga.bit.bin")
        .output()
        .expect("failed to execute command");

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

    let mut i2c = I2cdev::new("/dev/i2c-0")?;
    let addr: u8 = 0x68;
    let mut buf = [0u8; 2];
    i2c.write_read(addr, &[0x0F], &mut buf)
        .expect("Failed to i2c::write_read");
    println!("Read 0x0F → {:02X} {:02X}", buf[0], buf[1]);

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
}
