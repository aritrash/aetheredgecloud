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
// Added State here to resolve your E0433 errors
use smoltcp::socket::tcp::{Socket as TcpSocket, SocketBuffer as TcpSocketBuffer, State as TcpState};
use crate::drivers::framebuffer::Framebuffer;


// 1. Static buffers for the TCP connection (Global scope to ensure persistence)
static mut TCP_RX_DATA: [u8; 4096] = [0; 4096];
static mut TCP_TX_DATA: [u8; 4096] = [0; 4096];

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

            let (rx_q, tx_q) = unsafe { drivers::virtio_net::init(base) };

            let status = unsafe { drivers::virtio_net::check_device_status(base) };
            uart::puts("[VirtIO] Raw Status Register: ");
            uart::putc_hex64(status as u64);
            uart::puts("\n");

            if unsafe { !drivers::virtio_net::is_device_ready(base) } {
                uart::puts("[ERROR] NIC is NOT in DRIVER_OK state! Transmit will fail.\n");
            }
            
            device_opt = Some(net::VirtioNetDevice {
                base,
                rx: rx_q,
                tx: tx_q,
            });
            break;
        }
    }

    let mut device = match device_opt {
        Some(d) => d,
        None => {
            uart::puts("[WARN] No VirtIO Network Card found! Halting.\n");
            loop { unsafe { asm!("wfi"); } }
        }
    };

    // --- SMOLTCP INTERFACE SETUP ---
    let mut config = Config::new(EthernetAddress([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]).into());
    config.random_seed = 0x12345678;

    let mut iface = Interface::new(config, &mut device, Instant::from_millis(0));
    
    // FORCE LINK UP: Without this, smoltcp assumes the "cable" is unplugged
    // and won't even call the transmit function.
    iface.set_any_ip(true); 

    // Add this to ensure the stack doesn't try to offload checksums to VirtIO
    // (which we haven't implemented in the driver yet)
    // iface.set_hardware_checksum_offload(false); // Depending on your smoltcp version
    
    iface.update_ip_addrs(|addrs| {
        addrs.push(IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24)).unwrap();
    });

    // Static storage for sockets
    let mut socket_storage = [SocketStorage::EMPTY; 8];
    let mut sockets = SocketSet::new(&mut socket_storage[..]);

    // Setup TCP Socket
    let tcp_rx_buffer = TcpSocketBuffer::new(unsafe { &mut TCP_RX_DATA[..] });
    let tcp_tx_buffer = TcpSocketBuffer::new(unsafe { &mut TCP_TX_DATA[..] });
    let mut tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);
    
    tcp_socket.listen(80).unwrap();
    let tcp_handle = sockets.add(tcp_socket);

    uart::puts("[OK] Network Stack Initialized (IP: 10.0.2.15)\n");
    uart::puts("[OK] Aether Dashboard Service started on Port 80.\n");

    // 6. Global Interrupt Enable
    uart::puts("[CHECK] Unmasking External Interrupts...\n");
    unsafe { asm!("msr daifclr, #2"); }
    uart::puts("[OK] Interrupts Active.\n");

    // 7. Enable Generic Timer
    uart::puts("[CHECK] Setting up Heartbeat Timer...\n");
    unsafe {
        let cntfrq: u64;
        asm!("mrs {}, cntfrq_el0", out(reg) cntfrq);
        asm!("msr cntv_tval_el0, {}", in(reg) cntfrq);
        asm!("msr cntv_ctl_el0, {}", in(reg) 1u64);
    }

    uart::puts("\n--- Aether OS System Ready ---\n");

    let fb_base = 0x4000_0000; // we’ll adjust this once confirmed
    let fb = Framebuffer::new(fb_base, 1024, 768, 1024 * 4);

    fb.clear(0x00112233); // dark blue-ish

    // 8. THE SERVICE LOOP
    loop {
        let ms = arch::aarch64::get_current_time_ms();
        let timestamp = Instant::from_millis(ms as i64);

        iface.poll(timestamp, &mut device, &mut sockets);

        {
            let socket = sockets.get_mut::<TcpSocket>(tcp_handle);

            if socket.is_active() {

                if socket.can_recv() {
                    let mut buf = [0u8; 1024];

                    if let Ok(size) = socket.recv_slice(&mut buf) {
                        uart::puts("[NET] HTTP request received\n");

                        let response = net::dispatch_request(&buf[..size]);

                        if socket.may_send() {
                            socket.send_slice(response.header).ok();
                            socket.send_slice(response.body).ok();
                            uart::puts("[NET] HTTP response queued\n");
                        }
                    }
                }

            } else if socket.state() == TcpState::Closed {
                uart::puts("[NET] Re-listening on port 80\n");
                socket.listen(80).unwrap();
            }
        }

        iface.poll(Instant::from_millis((ms + 1) as i64), &mut device, &mut sockets);
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    uart::puts("\n!!!!!!!!!!!!!!!! KERNEL PANIC !!!!!!!!!!!!!!!!\n");
    if let Some(location) = info.location() {
        uart::puts("File: "); uart::puts(location.file());
        uart::puts(" Line: "); put_decimal(location.line() as u64);
    }
    loop { unsafe { asm!("wfe"); } }
}

fn put_decimal(mut n: u64) {
    if n == 0 { uart::putc(b'0'); return; }
    let mut buf = [0u8; 20];
    let mut i = 0;
    while n > 0 { buf[i] = (n % 10) as u8 + b'0'; n /= 10; i += 1; }
    for j in (0..i).rev() { uart::putc(buf[j]); }
}