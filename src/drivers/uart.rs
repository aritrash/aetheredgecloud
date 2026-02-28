use core::ptr::{read_volatile, write_volatile};

const UART_BASE: usize = 0x09000000;
const DR: usize = 0x00;
const FR: usize = 0x18;

#[inline(always)]
fn reg(offset: usize) -> *mut u32 {
    (UART_BASE + offset) as *mut u32
}

pub fn putc(c: u8) {
    unsafe {
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