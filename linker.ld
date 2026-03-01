ENTRY(_start)

SECTIONS
{
    /* QEMU Virt RAM starts at 0x40000000. 
       We load the kernel at 0x40080000 to leave space for 
       early page tables or bootloader data if needed. */
    . = 0x40080000;

    /* --- Code Section --- */
    .text : {
        KEEP(*(.text.boot)) /* Ensure _start is at the very beginning */
        *(.text .text.*)
    }

    /* --- Exception Vectors --- */
    /* ARMv8 requires the Vector Base Address (VBAR) to be 2048-byte aligned */
    . = ALIGN(2048);
    .vectors : {
        __vectors_el1 = .;
        KEEP(*(.vectors))
    }

    /* --- Read-Only Data --- */
    .rodata : {
        . = ALIGN(16);
        *(.rodata .rodata.*)
    }

    /* --- Initialized Data --- */
    .data : {
        . = ALIGN(16);
        *(.data .data.*)
    }

    /* --- Uninitialized Data (BSS) --- */
    .bss (NOLOAD) : {
        . = ALIGN(16);
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(16);
        __bss_end = .;
    }

    /* --- Stack Space --- */
    /* 16-byte alignment is a hardware requirement for AArch64 stacks */
    . = ALIGN(16);
    . += 0x8000; /* 32KB Stack for the Kernel */
    _stack_top = .;

    /* End of kernel marker */
    _end = .;
}