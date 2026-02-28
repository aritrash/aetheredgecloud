pub unsafe fn launch_guest() -> ! {
    use core::arch::asm;
    use core::ptr::copy_nonoverlapping;

    extern "C" {
        fn guest_entry();
    }

    // ----------------------------
    // Guest memory layout
    // ----------------------------
    let guest_region: u64 = 0x4020_0000;
    let guest_stack:  u64 = 0x4021_0000;

    // ----------------------------
    // Copy guest stub into RAM
    // ----------------------------
    let src = guest_entry as *const u8;
    let dst = guest_region as *mut u8;

    // Copy 256 bytes (stub is small)
    copy_nonoverlapping(src, dst, 256);

    // ----------------------------
    // Cache maintenance
    // ----------------------------
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

    // ----------------------------
    // Set EL1 stack pointer
    // ----------------------------
    asm!("msr sp_el1, {}", in(reg) guest_stack);

    // ----------------------------
    // Force EL1 exception vectors
    // to an unmapped address
    // so SVC cannot be handled locally
    // ----------------------------
    let invalid_vector: u64 = 0xFFFF_FFFF_FFFF_F000;
    asm!("msr vbar_el1, {}", in(reg) invalid_vector);
    asm!("isb");

    // ----------------------------
    // Set ELR_EL2 = guest entry
    // ----------------------------
    asm!("msr elr_el2, {}", in(reg) guest_region);

    // ----------------------------
    // Prepare SPSR_EL2:
    //  - EL1h mode (0b0101)
    //  - Mask DAIF interrupts
    // ----------------------------
    let spsr: u64 =
        (1 << 9) |  // D mask
        (1 << 8) |  // A mask
        (1 << 7) |  // I mask
        (1 << 6) |  // F mask
        0b0101;     // EL1h

    asm!("msr spsr_el2, {}", in(reg) spsr);
    asm!("isb");

    // ----------------------------
    // Enter EL1
    // ----------------------------
    asm!("eret");

    loop {}
}