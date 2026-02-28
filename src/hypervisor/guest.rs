pub unsafe fn launch_guest() -> ! {
    use core::arch::asm;

    extern "C" {
        fn guest_entry();
    }

    // ----------------------------
    // EL1 stack
    // ----------------------------
    // Pick a safe location in RAM.
    // Your RAM starts at 0x40000000 and you mapped 4GB,
    // so this is fine.
    let guest_stack: u64 = 0x4021_0000;

    // ----------------------------
    // Set SP_EL1
    // ----------------------------
    asm!("msr sp_el1, {}", in(reg) guest_stack);

    // ----------------------------
    // Set ELR_EL2 to actual guest_entry
    // (execute in place, no copying)
    // ----------------------------
    let entry_addr = guest_entry as *const () as u64;
    asm!("msr elr_el2, {}", in(reg) entry_addr);

    // ----------------------------
    // Prepare SPSR_EL2:
    //  - EL1h (0b0101)
    //  - Mask DAIF
    // ----------------------------
    let spsr: u64 =
        (1 << 9) |  // D
        (1 << 8) |  // A
        (1 << 7) |  // I
        (1 << 6) |  // F
        0b0101;     // EL1h

    asm!("msr spsr_el2, {}", in(reg) spsr);
    asm!("isb");

    // ----------------------------
    // Enter EL1
    // ----------------------------
    asm!("eret");

    loop {}
}