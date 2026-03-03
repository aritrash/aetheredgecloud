#![no_std]
#![no_main]

mod arch;
mod drivers;
mod gfx;
mod pci;   // <-- NEW

use drivers::uart;
use core::arch::asm;
use pci::host::qemu_virt::QemuVirtPci;
use pci::core::enumerate;

#[no_mangle]
pub extern "C" fn kmain() {
    uart::init();
    uart::puts("\x1B[2J\x1B[H");
    uart::puts("====================================================\n");
    uart::puts("       Aether EdgeCloud Monolithic Kernel v0.1      \n");
    uart::puts("====================================================\n");
    uart::puts("[CHECK] UART: Initialized\n");

    // ---------------- CPU INFO ----------------

    let current_el: u64;
    let midr: u64;
    unsafe {
        asm!("mrs {}, CurrentEL", out(reg) current_el);
        asm!("mrs {}, MIDR_EL1", out(reg) midr);
    }

    uart::puts("[INFO] Current EL: ");
    uart::putc_hex64(current_el >> 2);
    uart::puts("\n[INFO] CPU ID (MIDR): ");
    uart::putc_hex64(midr);
    uart::puts("\n");

    // ---------------- VBAR ----------------

    uart::puts("[CHECK] Setting up Exception Vectors...\n");
    unsafe {
        extern "C" { static __vectors_el1: u8; }
        let vbar = &__vectors_el1 as *const u8 as u64;
        asm!("msr vbar_el1, {}", in(reg) vbar);
        asm!("isb");
    }
    uart::puts("[OK] VBAR_EL1 set.\n");

    // ---------------- GIC ----------------

    uart::puts("[CHECK] Initializing GICv3...\n");
    drivers::gic::init();
    uart::puts("[OK] GICv3 Ready.\n");

    // ---------------- PCI INIT ----------------

    uart::puts("[CHECK] Initializing PCI subsystem...\n");

    unsafe {
        let host = QemuVirtPci;
        enumerate(&host);
    }

    uart::puts("[OK] PCI Enumeration Complete.\n");

    // ---------------- ENABLE IRQ ----------------

    unsafe { asm!("msr daifclr, #2"); }

    // ---------------- TIMER SETUP ----------------

    unsafe {
        let freq: u64;
        asm!("mrs {}, cntfrq_el0", out(reg) freq);
        asm!("msr cntv_tval_el0, {}", in(reg) freq);
        asm!("msr cntv_ctl_el0, {}", in(reg) 1u64);
    }

    uart::puts("\n--- Aether OS Ready (PCI Mode) ---\n");

    // ---------------- MAIN LOOP ----------------

    loop {
        unsafe { asm!("wfi"); }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    uart::puts("\n!!!! KERNEL PANIC !!!!\n");
    if let Some(location) = info.location() {
        uart::puts("File: ");
        uart::puts(location.file());
        uart::puts(" Line: ");
        put_decimal(location.line() as u64);
    }
    loop { unsafe { asm!("wfe"); } }
}

fn put_decimal(mut n: u64) {
    if n == 0 { uart::putc(b'0'); return; }
    let mut buf = [0u8; 20];
    let mut i = 0;
    while n > 0 {
        buf[i] = (n % 10) as u8 + b'0';
        n /= 10;
        i += 1;
    }
    for j in (0..i).rev() {
        uart::putc(buf[j]);
    }
}