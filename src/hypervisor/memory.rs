use core::arch::asm;

#[repr(C, align(4096))]
struct Stage2Table {
    entries: [u64; 512],
}

static mut STAGE2_L1: Stage2Table = Stage2Table {
    entries: [0; 512],
};

pub unsafe fn init_stage2() {
    //
    // Configure MAIR_EL2
    //
    // AttrIdx 0 = Normal memory, Write-back, Read/Write allocate
    // 0xFF = 0b11111111 (Normal WB WA RA)
    //
    let mair: u64 = 0xFF;
    asm!("msr mair_el2, {}", in(reg) mair);
    asm!("isb");

    //
    // Identity map first 4GB using 1GB L1 block entries
    //
    for i in 0..4 {
        let block_addr = (i as u64) << 30; // 1GB per entry

        let desc =
            (block_addr & 0x0000_FFFF_FFFF_F000) | // Output address
            (0b11 << 6) |   // S2AP: Read/Write
            (0b11 << 8) |   // SH: Inner Shareable
            (0b0000 << 2) | // AttrIdx = 0 (uses MAIR_EL2[7:0])
            0b01;           // Valid block descriptor

        STAGE2_L1.entries[i] = desc;
    }

    configure_el2();
}

unsafe fn configure_el2() {
    let table_addr = &raw const STAGE2_L1 as *const _ as u64;

    //
    // Set VTTBR_EL2
    //
    asm!("msr vttbr_el2, {}", in(reg) table_addr);

    //
    // Configure VTCR_EL2
    //
    // TG0  = 0b00 → 4KB granule
    // PS   = 0b10 → 40-bit physical address
    // SL0  = 0b00 → Start at level 1
    // T0SZ = 24   → 40-bit IPA (64 - 40)
    //
    let vtcr: u64 =
        (0b00 << 14) |   // TG0: 4KB
        (0b10 << 16) |   // PS: 40-bit PA
        (0b01 << 6)  |   // SL0: L1
        (24 << 0);       // T0SZ: 40-bit IPA

    asm!("msr vtcr_el2, {}", in(reg) vtcr);
    asm!("isb");

    //
    // Invalidate all Stage-2 TLB entries
    //
    asm!("tlbi alle2");
    asm!("dsb sy");
    asm!("isb");

    //
    // Enable Stage-2 translation (HCR_EL2.VM)
    //
    let mut hcr: u64;
    asm!("mrs {}, hcr_el2", out(reg) hcr);
    hcr |= 1; // VM bit
    asm!("msr hcr_el2, {}", in(reg) hcr);
    asm!("isb");
}