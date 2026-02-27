pub unsafe fn launch_guest() -> ! {
    use core::arch::asm;
    use core::ptr::{copy_nonoverlapping};

    extern "C" {
        fn guest_entry();
    }

    let guest_region: u64 = 0x4020_0000;
    let guest_stack: u64 = 0x4021_0000;

    // Copy guest stub into guest memory
    let src = guest_entry as *const u8;
    let dst = guest_region as *mut u8;

    // Copy first 256 bytes (more than enough for stub)
    copy_nonoverlapping(src, dst, 256);

    // Set SP_EL1
    asm!("msr sp_el1, {}", in(reg) guest_stack);

    // Set ELR_EL2 to guest memory region
    asm!("msr elr_el2, {}", in(reg) guest_region);

    // EL1h + mask interrupts
    let spsr: u64 =
        (1 << 9) |
        (1 << 8) |
        (1 << 7) |
        (1 << 6) |
        0b0101;

    asm!("msr spsr_el2, {}", in(reg) spsr);

    for offset in (0..256).step_by(64) {
        let addr = guest_region + offset;
        asm!("dc cvau, {}", in(reg) addr);
    }
    asm!("dsb ish");

    for offset in (0..256).step_by(64) {
        let addr = guest_region + offset;
        asm!("ic ivau, {}", in(reg) addr);
    }
    asm!("dsb ish");
    asm!("isb");

    asm!("eret");

    loop {}
}