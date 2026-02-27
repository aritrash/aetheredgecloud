core::arch::global_asm!(
r#"
.section .text.boot
.global _start

_start:
    // Read CurrentEL
    mrs x0, CurrentEL
    lsr x0, x0, #2

    // If not EL2, hang
    cmp x0, #2
    b.ne hang

    // Setup stack
    ldr x0, =_stack_top
    mov sp, x0

    // Clear BSS
    ldr x0, =__bss_start
    ldr x1, =__bss_end

clear_bss:
    cmp x0, x1
    b.ge bss_done
    str xzr, [x0], #8
    b clear_bss

bss_done:
    ldr x0, =__vectors_el2
    msr vbar_el2, x0
    isb
    bl rust_main

hang:
    wfe
    b hang
"#
);