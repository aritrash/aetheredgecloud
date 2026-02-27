#![no_std]
#![no_main]

mod arch;
mod drivers;

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    drivers::uart::puts("Aether EdgeCloud EL2 Booted\n");

    let el: u64;
    unsafe {
        core::arch::asm!("mrs {}, CurrentEL", out(reg) el);
    }

    let el_level = el >> 2;

    drivers::uart::puts("CurrentEL = ");
    drivers::uart::putc(b'0' + el_level as u8);
    drivers::uart::puts("\n");

    unsafe {
        core::arch::asm!("svc #0");
    }
    
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}