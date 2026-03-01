use core::arch::asm;

core::arch::global_asm!(
r#"
.section .vectors, "ax"
.align 11
.global __vectors_el1

__vectors_el1:
    /* Current EL with SP0 */
    .align 7; b . 
    .align 7; b .
    .align 7; b .
    .align 7; b .

    /* Current EL with SPx (Kernel interrupts) */
    .align 7; b handle_sync_el1
    .align 7; b handle_irq_el1
    .align 7; b .
    .align 7; b .

    /* Lower EL AArch64 */
    .align 7; b .
    .align 7; b .
    .align 7; b .
    .align 7; b .

    /* Lower EL AArch32 */
    .align 7; b .
    .align 7; b .
    .align 7; b .
    .align 7; b .

handle_sync_el1:
    sub sp, sp, #272
    stp x0, x1, [sp, #0]
    stp x2, x3, [sp, #16]
    stp x4, x5, [sp, #32]
    stp x6, x7, [sp, #48]
    stp x8, x9, [sp, #64]
    stp x10, x11, [sp, #80]
    stp x12, x13, [sp, #96]
    stp x14, x15, [sp, #112]
    stp x16, x17, [sp, #128]
    stp x18, x19, [sp, #144]
    stp x20, x21, [sp, #160]
    stp x22, x23, [sp, #176]
    stp x24, x25, [sp, #192]
    stp x26, x27, [sp, #208]
    stp x28, x29, [sp, #224]
    str x30, [sp, #240]

    mov x0, sp
    bl rust_sync_handler

    ldr x30, [sp, #240]
    ldp x28, x29, [sp, #224]
    ldp x26, x27, [sp, #208]
    ldp x24, x25, [sp, #192]
    ldp x22, x23, [sp, #176]
    ldp x20, x21, [sp, #160]
    ldp x18, x19, [sp, #144]
    ldp x16, x17, [sp, #128]
    ldp x14, x15, [sp, #112]
    ldp x12, x13, [sp, #96]
    ldp x10, x11, [sp, #80]
    ldp x8, x9, [sp, #64]
    ldp x6, x7, [sp, #48]
    ldp x4, x5, [sp, #32]
    ldp x2, x3, [sp, #16]
    ldp x0, x1, [sp, #0]
    add sp, sp, #272
    eret

handle_irq_el1:
    sub sp, sp, #272
    stp x0, x1, [sp, #0]
    stp x2, x3, [sp, #16]
    stp x4, x5, [sp, #32]
    stp x6, x7, [sp, #48]
    stp x8, x9, [sp, #64]
    stp x10, x11, [sp, #80]
    stp x12, x13, [sp, #96]
    stp x14, x15, [sp, #112]
    stp x16, x17, [sp, #128]
    stp x18, x19, [sp, #144]
    stp x20, x21, [sp, #160]
    stp x22, x23, [sp, #176]
    stp x24, x25, [sp, #192]
    stp x26, x27, [sp, #208]
    stp x28, x29, [sp, #224]
    str x30, [sp, #240]

    mov x0, sp
    bl rust_irq_handler

    ldr x30, [sp, #240]
    ldp x28, x29, [sp, #224]
    ldp x26, x27, [sp, #208]
    ldp x24, x25, [sp, #192]
    ldp x22, x23, [sp, #176]
    ldp x20, x21, [sp, #160]
    ldp x18, x19, [sp, #144]
    ldp x16, x17, [sp, #128]
    ldp x14, x15, [sp, #112]
    ldp x12, x13, [sp, #96]
    ldp x10, x11, [sp, #80]
    ldp x8, x9, [sp, #64]
    ldp x6, x7, [sp, #48]
    ldp x4, x5, [sp, #32]
    ldp x2, x3, [sp, #16]
    ldp x0, x1, [sp, #0]
    add sp, sp, #272
    eret
"#
);

#[no_mangle]
pub extern "C" fn rust_sync_handler() {
    let esr: u64;
    let far: u64;
    unsafe {
        asm!("mrs {}, esr_el1", out(reg) esr);
        asm!("mrs {}, far_el1", out(reg) far);
    }
    crate::drivers::uart::puts("\n--- PANIC: SYNC EXCEPTION ---\n");
    crate::drivers::uart::puts("ESR_EL1: "); crate::drivers::uart::putc_hex64(esr);
    crate::drivers::uart::puts("\nFAR_EL1: "); crate::drivers::uart::putc_hex64(far);
    crate::drivers::uart::puts("\n----------------------------\n");
    loop {}
}

#[no_mangle]
pub extern "C" fn rust_irq_handler() {
    let irq = crate::drivers::gic::acknowledge_irq();

    // IRQ 30 is the standard AArch64 Virtual Timer
    if irq == 30 {
        crate::drivers::uart::puts("\n[HEARTBEAT] Tick! Timer interrupt received.\n");
        
        // Reset the timer for the next 1 second
        unsafe {
            let freq: u64;
            core::arch::asm!("mrs {}, cntfrq_el0", out(reg) freq);
            core::arch::asm!("msr cntv_tval_el0, {}", in(reg) freq);
        }
    } else if irq >= 48 && irq <= 96 { // Likely VirtIO range
        crate::drivers::uart::puts("\n[NET] Network Packet Received!\n");
        unsafe { crate::drivers::virtio_net::handle_interrupt(); }
    } else if irq < 1023 {
        crate::drivers::uart::puts("\n[IRQ] External interrupt: ");
        crate::drivers::uart::putc_hex64(irq as u64);
        crate::drivers::uart::puts("\n");
    }

    // Signal the GIC that we are done with this IRQ
    crate::drivers::gic::end_of_interrupt(irq);
}