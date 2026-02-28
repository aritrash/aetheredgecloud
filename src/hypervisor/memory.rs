use core::arch::asm;

#[repr(C, align(4096))]
struct Stage2Table {
    entries: [u64; 512],
}

// Level 0 (root)
static mut STAGE2_L0: Stage2Table = Stage2Table {
    entries: [0; 512],
};

// Level 1
static mut STAGE2_L1: Stage2Table = Stage2Table {
    entries: [0; 512],
};

// Level 2 table for first 1GB
static mut STAGE2_L2_0: Stage2Table = Stage2Table {
    entries: [0; 512],
};

pub unsafe fn init_stage2() {
    // MAIR (not strictly needed for Stage-2, but harmless)
    let mair: u64 = 0xFF;
    asm!("msr mair_el2, {}", in(reg) mair);
    asm!("isb");

    //
    // ---------------------------
    // L2 for first 1GB (2MB blocks)
    // All mapped as Normal WB
    // ---------------------------
    //
    for i in 0..512 {
        let block_addr = (i as u64) << 21; // 2MB aligned

        let desc =
            (block_addr & 0x0000_FFFF_FFE0_0000) | // bits[47:21]
            (0b1111u64 << 2) |   // MemAttr = Normal WB
            (0b11u64 << 6)  |    // S2AP = RW
            (0b11u64 << 8)  |    // Inner Shareable
            (1u64 << 10)    |    // AF
            0b01u64;             // Valid block

        STAGE2_L2_0.entries[i] = desc;
    }

    //
    // ---------------------------
    // L1 setup
    // ---------------------------
    //

    // L1[0] → table descriptor to L2
    let l2_addr = &raw const STAGE2_L2_0 as *const _ as u64;
    STAGE2_L1.entries[0] =
        (l2_addr & 0x0000_FFFF_FFFF_F000) | 0b11u64;

    // L1[1..4] → 1GB Normal blocks
    for i in 1..4 {
        let block_addr = (i as u64) << 30;

        let desc =
            (block_addr & 0x0000_FFFF_C000_0000) |
            (0b1111u64 << 2) |   // Normal WB
            (0b11u64 << 6)  |    // S2AP RW
            (0b11u64 << 8)  |    // Inner Shareable
            (1u64 << 10)    |    // AF
            0b01u64;

        STAGE2_L1.entries[i] = desc;
    }

    //
    // ---------------------------
    // L0 root
    // ---------------------------
    //
    let l1_addr = &raw const STAGE2_L1 as *const _ as u64;
    STAGE2_L0.entries[0] =
        (l1_addr & 0x0000_FFFF_FFFF_F000) | 0b11u64;

    configure_el2();
}

unsafe fn configure_el2() {
    let l0_addr = &raw const STAGE2_L0 as *const _ as u64;

    asm!("msr vttbr_el2, {}", in(reg) l0_addr);

    let vtcr: u64 =
        (0b00 << 14) |   // TG0 = 4KB
        (0b10 << 16) |   // PS = 40-bit PA
        (0b10 << 6)  |   // SL0 = L0
        (24 << 0);       // T0SZ = 40-bit IPA

    asm!("msr vtcr_el2, {}", in(reg) vtcr);
    asm!("isb");

    asm!("tlbi alle2");
    asm!("dsb sy");
    asm!("isb");

    let mut hcr: u64;
    asm!("mrs {}, hcr_el2", out(reg) hcr);

    hcr |= 1 << 0;   // VM
    hcr |= 1 << 31;  // EL1 AArch64
    hcr |= 1 << 19;   // TSC

    asm!("msr hcr_el2, {}", in(reg) hcr);
    asm!("isb");
}