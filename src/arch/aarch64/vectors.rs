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
    eret

handle_irq:
    bl rust_el2_exception
    eret

handle_fiq:
    bl rust_el2_exception
    eret

handle_serr:
    bl rust_el2_exception
    eret
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

    let ec = (esr >> 26) & 0x3F;

    if ec == 0x16 {
        crate::drivers::uart::puts("[EL2] SVC trapped from EL1\n");

        // Advance ELR_EL2 past SVC instruction
        let new_elr = elr + 4;

        unsafe {
            core::arch::asm!("msr elr_el2, {}", in(reg) new_elr);
        }

        return;
    }

    crate::drivers::uart::puts("\n=== EL2 EXCEPTION ===\n");

    crate::drivers::uart::puts("ESR_EL2 = 0x");
    crate::drivers::uart::putc_hex64(esr);

    crate::drivers::uart::puts("\nELR_EL2 = 0x");
    crate::drivers::uart::putc_hex64(elr);

    crate::drivers::uart::puts("\nFAR_EL2 = 0x");
    crate::drivers::uart::putc_hex64(far);

    crate::drivers::uart::puts("\n");

    loop {}
}