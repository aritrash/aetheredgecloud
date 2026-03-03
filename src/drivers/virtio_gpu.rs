use crate::drivers::uart;

pub fn init() {
    uart::puts("[GPU] VirtIO MMIO driver disabled. PCI GPU not implemented yet.\n");
}