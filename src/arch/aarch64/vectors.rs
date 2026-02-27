core::arch::global_asm!(
r#"
.section .vectors, "ax"
.align 11
.global __vectors_el2

__vectors_el2:

// 0x000 Sync current EL (SP0)
.align 7
b handle_sync

// 0x080 IRQ current EL (SP0)
.align 7
b handle_irq

// 0x100 FIQ current EL (SP0)
.align 7
b handle_fiq

// 0x180 SError current EL (SP0)
.align 7
b handle_serr

// 0x200 Sync current EL (SPx)
.align 7
b handle_sync

// 0x280 IRQ current EL (SPx)
.align 7
b handle_irq

// 0x300 FIQ current EL (SPx)
.align 7
b handle_fiq

// 0x380 SError current EL (SPx)
.align 7
b handle_serr

// 0x400 Sync lower EL AArch64
.align 7
b handle_sync

// 0x480 IRQ lower EL AArch64
.align 7
b handle_irq

// 0x500 FIQ lower EL AArch64
.align 7
b handle_fiq

// 0x580 SError lower EL AArch64
.align 7
b handle_serr

// 0x600 Sync lower EL AArch32
.align 7
b handle_sync

// 0x680 IRQ lower EL AArch32
.align 7
b handle_irq

// 0x700 FIQ lower EL AArch32
.align 7
b handle_fiq

// 0x780 SError lower EL AArch32
.align 7
b handle_serr

handle_sync:
    bl rust_el2_exception
1:  wfe
    b 1b

handle_irq:
    bl rust_el2_exception
1:  wfe
    b 1b

handle_fiq:
    bl rust_el2_exception
1:  wfe
    b 1b

handle_serr:
    bl rust_el2_exception
1:  wfe
    b 1b
"#
);

#[no_mangle]
pub extern "C" fn rust_el2_exception() {
    let esr: u64;
    let elr: u64;
    let far: u64;

    unsafe {
        core::arch::asm!("mrs {}, esr_el2", out(reg) esr);
        core::arch::asm!("mrs {}, elr_el2", out(reg) elr);
        core::arch::asm!("mrs {}, far_el2", out(reg) far);
    }

    crate::drivers::uart::puts("\n=== EL2 EXCEPTION ===\n");
    crate::drivers::uart::puts("ESR_EL2 = ");
    print_hex(esr);
    crate::drivers::uart::puts("\nELR_EL2 = ");
    print_hex(elr);
    crate::drivers::uart::puts("\nFAR_EL2 = ");
    print_hex(far);
    crate::drivers::uart::puts("\n");

    loop {}
}

fn print_hex(val: u64) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    crate::drivers::uart::puts("0x");
    for i in (0..16).rev() {
        let nibble = ((val >> (i * 4)) & 0xF) as usize;
        crate::drivers::uart::putc(HEX[nibble]);
    }
}