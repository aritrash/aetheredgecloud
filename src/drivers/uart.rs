use core::ptr::{read_volatile, write_volatile};

const UART_BASE: usize = 0x09000000;

// PL011 Register Offsets
const DR:   usize = 0x00; // Data Register
const FR:   usize = 0x18; // Flag Register
const IBRD: usize = 0x24; // Integer Baud Rate
const FBRD: usize = 0x28; // Fractional Baud Rate
const LCRH: usize = 0x2C; // Line Control Register
const CR:   usize = 0x30; // Control Register
const IMSC: usize = 0x38; // Interrupt Mask Set/Clear

#[inline(always)]
fn reg(offset: usize) -> *mut u32 {
    (UART_BASE + offset) as *mut u32
}

pub fn init() {
    unsafe {
        // 1. Disable UART while configuring
        write_volatile(reg(CR), 0);

        // 2. Set Baud Rate (Assuming 24MHz clock for 115200 baud)
        // IBRD = 24,000,000 / (16 * 115200) = 13.020833
        // FBRD = 0.020833 * 64 + 0.5 = 1.83 (use 1)
        write_volatile(reg(IBRD), 13);
        write_volatile(reg(FBRD), 1);

        // 3. Line Control: 8 bits, no parity, 1 stop bit, FIFO enabled (bit 4)
        write_volatile(reg(LCRH), (0b11 << 5) | (1 << 4));

        // 4. Mask all interrupts for now
        write_volatile(reg(IMSC), 0);

        // 5. Enable UART, Transmit, and Receive (bits 0, 8, 9)
        write_volatile(reg(CR), (1 << 0) | (1 << 8) | (1 << 9));
    }
}

pub fn putc(c: u8) {
    unsafe {
        // Wait until TX FIFO is not full (FR bit 5)
        while read_volatile(reg(FR)) & (1 << 5) != 0 {}
        write_volatile(reg(DR), c as u32);
    }
}

pub fn puts(s: &str) {
    for b in s.bytes() {
        if b == b'\n' {
            putc(b'\r');
        }
        putc(b);
    }
}

pub fn putc_hex64(val: u64) {
    for i in (0..16).rev() {
        let nibble = ((val >> (i * 4)) & 0xF) as u8;
        let c = match nibble {
            0..=9 => b'0' + nibble,
            _ => b'A' + (nibble - 10),
        };
        putc(c);
    }
}