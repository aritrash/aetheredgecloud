#![no_std]
#![no_main]

mod arch;
mod drivers;
mod net;

use drivers::uart;
use core::arch::asm;

// --- SMOLTCP IMPORTS ---
use smoltcp::iface::{Config, Interface, SocketSet, SocketStorage};
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr};
use smoltcp::time::Instant;

#[no_mangle]
pub extern "C" fn kmain() {
    // 1. Hardware Early Setup
    uart::init();
    uart::puts("\x1B[2J\x1B[H"); // ANSI Clear screen & Home
    uart::puts("====================================================\n");
    uart::puts("       Aether EdgeCloud Monolithic Kernel v0.1      \n");
    uart::puts("====================================================\n");
    uart::puts("[CHECK] UART: Initialized\n");

    // 2. Identify Privilege Level & Architecture
    let current_el: u64;
    let midr: u64;
    unsafe { 
        asm!("mrs {}, CurrentEL", out(reg) current_el); 
        asm!("mrs {}, MIDR_EL1", out(reg) midr);
    }
    
    uart::puts("[INFO] Current EL: ");
    uart::putc_hex64(current_el >> 2);
    uart::puts("\n[INFO] CPU ID (MIDR): ");
    uart::putc_hex64(midr);
    uart::puts("\n");

    // 3. Exception Vector Setup
    uart::puts("[CHECK] Setting up Exception Vectors...\n");
    unsafe {
        extern "C" { static __vectors_el1: u8; }
        let vbar = &__vectors_el1 as *const u8 as u64;
        
        if vbar & 0x7FF != 0 {
            uart::puts("[ERROR] VBAR is NOT 2048-byte aligned! Base: ");
            uart::putc_hex64(vbar);
            uart::puts("\n");
        }
        
        asm!("msr vbar_el1, {}", in(reg) vbar);
        asm!("isb");
    }
    uart::puts("[OK] VBAR_EL1 set to vectors.\n");

    // 4. Interrupt Controller (GICv3) Initialization
    uart::puts("[CHECK] Initializing GICv3 (System Registers)...\n");
    drivers::gic::init();
    uart::puts("[OK] GICv3 Ready.\n");

    // 5. VirtIO Device Discovery & Network Stack Init
    uart::puts("[CHECK] Probing VirtIO MMIO slots...\n");
    
    let mut device_opt = None;

    for i in 0..32 {
        let base = 0x0a000000 + (i * 0x200);
        if unsafe { drivers::virtio_net::probe(base) } {
            uart::puts("[OK] Identified VirtIO Network Card. Initializing...\n");
            
            // init() now returns (rx_q, tx_q)
            let (rx_q, tx_q) = unsafe { drivers::virtio_net::init(base) };
            
            device_opt = Some(net::VirtioNetDevice {
                base,
                rx: rx_q,
                tx: tx_q,
            });
            break;
        }
    }

    // Unwrap the device or halt if no NIC found
    let mut device = match device_opt {
        Some(d) => d,
        None => {
            uart::puts("[WARN] No VirtIO Network Card found! Halting.\n");
            loop { unsafe { asm!("wfi"); } }
        }
    };

    // --- SMOLTCP INTERFACE SETUP ---
    // MAC: 52:54:00:12:34:56 (Standard QEMU test MAC)
    let config = Config::new(EthernetAddress([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]).into());
    let mut iface = Interface::new(config, &mut device, Instant::from_millis(0));
    
    iface.update_ip_addrs(|addrs| {
        // Set IP to 10.0.2.15 (QEMU User-net default)
        addrs.push(IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24)).unwrap();
    });

    // Static storage for sockets (No heap version)
    let mut socket_storage = [SocketStorage::EMPTY; 8];
    let mut sockets = SocketSet::new(&mut socket_storage[..]);

    uart::puts("[OK] Network Stack Initialized (IP: 10.0.2.15)\n");

    // 6. Global Interrupt Enable
    uart::puts("[CHECK] Unmasking External Interrupts...\n");
    unsafe { asm!("msr daifclr, #2"); }
    uart::puts("[OK] Interrupts Active.\n");

    // 7. Enable Generic Timer (IRQ 30) for heartbeat
    uart::puts("[CHECK] Setting up Heartbeat Timer...\n");
    unsafe {
        let cntfrq: u64;
        asm!("mrs {}, cntfrq_el0", out(reg) cntfrq);
        asm!("msr cntv_tval_el0, {}", in(reg) cntfrq);
        asm!("msr cntv_ctl_el0, {}", in(reg) 1u64);
    }
    uart::puts("[OK] Timer programmed for 1s intervals.\n");

    uart::puts("\n--- Aether OS System Ready ---\n");

    // 8. THE SERVICE LOOP
    loop {
        // Get monotonic time in ms for smoltcp
        let timestamp = Instant::from_millis(arch::aarch64::get_current_time_ms() as i64);

        // Process packets (Poll the stack)
        // This bool indicates if the interface changed state
        iface.poll(timestamp, &mut device, &mut sockets);

        // Here is where we'll eventually check TCP sockets for HTTP requests
    }
}

/// A tiny decimal printer for debugging line numbers
fn put_decimal(mut n: u64) {
    if n == 0 {
        uart::putc(b'0');
        return;
    }
    let mut buf = [0u8; 20];
    let mut i = 0;
    while n > 0 {
        buf[i] = (n % 10) as u8 + b'0';
        n /= 10;
        i += 1;
    }
    for j in (0..i).rev() {
        uart::putc(buf[j]);
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    uart::puts("\n!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n");
    uart::puts("                  KERNEL PANIC                      \n");
    uart::puts("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n");
    
    if let Some(location) = info.location() {
        uart::puts("File: ");
        uart::puts(location.file());
        uart::puts("\nLine: ");
        put_decimal(location.line() as u64);
    }
    
    uart::puts("\nReason: Code logic exception.");
    uart::puts("\nHalting CPU.\n");
    loop {
        unsafe { asm!("wfe"); }
    }
}