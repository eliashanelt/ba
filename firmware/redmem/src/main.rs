use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::process::Command;
use std::ptr::{read_volatile, write_volatile};
use std::time::Duration;

const FPGA_BASE_ADDR: usize = 0x40000000; // Adjust based on your AXI address
const PAGE_SIZE: usize = 4096;
const FREQ_HZ: f64 = 125_000_000.0; // 125 MHz

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
    assert!(
        (-8192..=8191).contains(&x),
        "Input must be in [-8192, 8191], got {x}",
    );
    if x > 0 {
        x as f32 / 8191.0
    } else if x < 0 {
        x as f32 / 8192.0
    } else {
        0.0
    }
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
            "min must be between -10.0 and 10.0, got {gain}",
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

// Example usage
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create interface (adjust address based on your Vivado design)
    flash_fpga_with_fpgautil("system_wrapper.bit.bin")?;
    let mut rp_interface = RedPitayaInterface::new(FPGA_BASE_ADDR)?;

    // Write values to FPGA
    rp_interface.set_limits(-0.5, 1.0); // Example limit value
    rp_interface.set_gain(-0.25); // Example gain value (can be negative)
    rp_interface.set_delay(50); // Example delay value

    // Read values from FPGA
    loop {
        let frequency = rp_interface.get_frequency();
        let vpp = rp_interface.get_vpp();
        let vp = rp_interface.get_vp();

        println!("Frequency: {frequency} Hz");
        println!("Vpp: {vpp}");
        println!("Vp: {vp}");
        std::thread::sleep(Duration::from_secs(3));
    }
}

// Alternative: Using a struct-based approach
#[repr(C)]
pub struct MemoryRegisters {
    pub frequency: u32,
    pub vpp: u32,
    pub vp: u32,
    pub limits: u32,
    pub gain: i32,
    pub delay: u32,
}

pub struct RedPitayaInterfaceAlt {
    registers: *mut MemoryRegisters,
    _mmap: MmapMut, // Keep mmap alive
}

impl RedPitayaInterfaceAlt {
    pub fn new(base_address: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let file = OpenOptions::new().read(true).write(true).open("/dev/mem")?;

        let page_addr = base_address & !(PAGE_SIZE - 1);
        let page_offset = base_address - page_addr;

        let mmap = unsafe {
            MmapOptions::new()
                .offset(page_addr as u64)
                .len(PAGE_SIZE)
                .map_mut(&file)?
        };

        let registers = unsafe { mmap.as_ptr().add(page_offset) as *mut MemoryRegisters };

        Ok(RedPitayaInterfaceAlt {
            registers,
            _mmap: mmap,
        })
    }

    pub fn registers(&mut self) -> &mut MemoryRegisters {
        unsafe { &mut *self.registers }
    }
}
