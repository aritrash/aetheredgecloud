#![no_std]
#![no_main]

mod arch;
mod drivers;
mod hypervisor;
mod el1;

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    drivers::uart::puts("[OK] Aether EdgeCloud EL2 Booted\n");

    let el: u64;
    unsafe {
        core::arch::asm!("mrs {}, CurrentEL", out(reg) el);
    }

    let el_level = el >> 2;

    drivers::uart::puts("[INFO] CurrentEL = ");
    drivers::uart::putc(b'0' + el_level as u8);
    drivers::uart::puts("\n");

    unsafe {
        hypervisor::memory::init_stage2();
    }

    drivers::uart::puts("[OK] Stage-2 Enabled\n");

    unsafe {
        hypervisor::guest::launch_guest();
    }
    
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}