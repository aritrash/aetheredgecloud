use core::ptr::{read_volatile, write_volatile};
use crate::drivers::uart;

const VIRTIO_BASE: usize = 0x0A000000;

const MAGIC_VALUE: usize   = 0x000;
const VERSION: usize       = 0x004;
const DEVICE_ID: usize     = 0x008;
const VENDOR_ID: usize     = 0x00C;

#[inline(always)]
fn reg(offset: usize) -> *mut u32 {
    (VIRTIO_BASE + offset) as *mut u32
}

pub fn probe() {
    unsafe {
        let magic  = read_volatile(reg(MAGIC_VALUE));
        let ver    = read_volatile(reg(VERSION));
        let devid  = read_volatile(reg(DEVICE_ID));
        let vendor = read_volatile(reg(VENDOR_ID));

        uart::puts("\n[VirtIO] Probing...\n");

        uart::puts("Magic: 0x");
        uart::putc_hex64(magic as u64);
        uart::puts("\nVersion: 0x");
        uart::putc_hex64(ver as u64);
        uart::puts("\nDeviceID: 0x");
        uart::putc_hex64(devid as u64);
        uart::puts("\nVendorID: 0x");
        uart::putc_hex64(vendor as u64);
        uart::puts("\n");
    }
}