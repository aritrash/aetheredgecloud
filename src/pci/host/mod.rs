pub mod qemu_virt;

pub trait PciHost {
    unsafe fn read(&self, bus: u8, dev: u8, func: u8, reg: u16) -> u32;
    unsafe fn write(&self, bus: u8, dev: u8, func: u8, reg: u16, val: u32);
}