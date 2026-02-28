core::arch::global_asm!(
r#"
.section .text.guest
.global guest_entry

guest_entry:
    mov x0, #'G'
    bl guest_uart_putc

    bl el1_rust_main

1:
    wfe
    b 1b

guest_uart_putc:
    ldr x1, =0x09000000
wait:
    ldr w2, [x1, #0x18]
    tbz w2, #5, send
    b wait
send:
    str w0, [x1, #0x00]
    ret
"#
);

extern "C" {
    pub fn guest_entry();
}