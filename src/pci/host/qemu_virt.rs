use core::ptr::{read_volatile, write_volatile};
use super::PciHost;

const ECAM_BASE: usize = 0x3f000000; // adjust if needed

pub struct QemuVirtPci;

impl PciHost for QemuVirtPci {
    unsafe fn read(&self, bus: u8, dev: u8, func: u8, reg: u16) -> u32 {
        let offset =
            ((bus as usize) << 20) |
            ((dev as usize) << 15) |
            ((func as usize) << 12) |
            ((reg as usize) & 0xFFC);

        let addr = (ECAM_BASE + offset) as *const u32;
        read_volatile(addr)
    }

    unsafe fn write(&self, bus: u8, dev: u8, func: u8, reg: u16, val: u32) {
        let offset =
            ((bus as usize) << 20) |
            ((dev as usize) << 15) |
            ((func as usize) << 12) |
            ((reg as usize) & 0xFFC);

        let addr = (ECAM_BASE + offset) as *mut u32;
        write_volatile(addr, val);
    }
}