core::arch::global_asm!(
r#"
.section .text.boot, "ax"
.global _start

_start:
    /* 1. Mask all interrupts immediately */
    msr daifset, #0xf

    /* 2. Check current Exception Level */
    mrs x0, CurrentEL
    lsr x0, x0, #2
    cmp x0, #2
    b.eq drop_to_el1
    b   setup_el1_state

drop_to_el1:
    /* Configure HCR_EL2: Set RW=1 (EL1 is AArch64) */
    mov x0, #(1 << 31)
    msr hcr_el2, x0
    
    /* Disable SIMD/FPU traps for EL1 */
    msr cptr_el2, xzr
    
    /* Set return address to setup_el1_state */
    adr x0, setup_el1_state
    msr elr_el2, x0
    
    /* Set SPSR_EL2: EL1h (SP_EL1), DAIF masked (0x3c5) */
    mov x0, #0x3c5
    msr spsr_el2, x0
    
    eret

setup_el1_state:
    /* 3. Enable SIMD/FPU in EL1 (Strictly required for Rust) */
    mov x0, #(3 << 20)
    msr cpacr_el1, x0
    isb

    /* 4. Clear BSS (Zero out uninitialized global variables) */
    ldr x0, =__bss_start
    ldr x1, =__bss_end
    sub x1, x1, x0
    cbz x1, setup_stack    /* Skip if BSS is empty */

clear_bss_loop:
    str xzr, [x0], #8      /* Store zero and increment x0 by 8 */
    subs x1, x1, #8        /* Decrement counter */
    b.gt clear_bss_loop

setup_stack:
    /* 5. Setup Stack Pointer (16-byte aligned) */
    ldr x0, =_stack_top
    mov sp, x0

    /* 6. Jump to Rust kmain */
    bl kmain

hang:
    wfe
    b hang
"#
);