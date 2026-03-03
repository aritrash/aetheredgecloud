use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{self, Ordering};

use crate::drivers::uart;
use crate::drivers::virtio_queue::VirtQueue;
use crate::drivers::virtio_mmio;

//
// =======================
//  VIRTIO NET CONSTANTS
// =======================
//

pub static mut NET_BASE: usize = 0;
const DEVICE_ID_NET: u32 = 1;

//
// =======================
//  PROBE
// =======================
//

pub unsafe fn probe(base: usize) -> bool {
    virtio_mmio::probe(base, DEVICE_ID_NET)
}

//
// =======================
//  INIT
// =======================
//

pub unsafe fn init(base: usize) -> (VirtQueue, VirtQueue) {
    uart::puts("[VirtIO-NET] Initializing...\n");

    unsafe {
        NET_BASE = base;
    }

    // Generic VirtIO handshake
    virtio_mmio::init_device(base);

    // Setup RX and TX queues
    let mut rx_q = virtio_mmio::setup_queue(base, 0);
    let tx_q      = virtio_mmio::setup_queue(base, 1);

    prime_rx_queue(base, &mut rx_q);

    uart::puts("[VirtIO-NET] Ready.\n");

    (rx_q, tx_q)
}

//
// =======================
//  PRIME RX QUEUE
// =======================
//

pub unsafe fn prime_rx_queue(base: usize, rx_q: &mut VirtQueue) {
    uart::puts("[VirtIO-NET] Priming RX queue...\n");

    for i in 0..rx_q.size {
        let buf = crate::drivers::allocator::allocate_aligned(1536, 16);

        // VIRTQ_DESC_F_WRITE = 2
        rx_q.add_desc(i, buf as u64, 1536, 2);

        (*rx_q.avail).ring[i as usize] = i;
    }

    atomic::fence(Ordering::SeqCst);

    (*rx_q.avail).idx = rx_q.size;

    atomic::fence(Ordering::SeqCst);

    virtio_mmio::notify_queue(base, 0);

    uart::puts("[VirtIO-NET] RX queue primed.\n");
}

//
// =======================
//  STATUS HELPERS
// =======================
//

pub unsafe fn check_device_status(base: usize) -> u32 {
    read_volatile((base + virtio_mmio::REG_STATUS) as *const u32)
}

pub unsafe fn is_device_ready(base: usize) -> bool {
    let status = check_device_status(base);
    (status & 0xF) == 0xF
}

//
// =======================
//  INTERRUPT HANDLER
// =======================
//

#[no_mangle]
pub unsafe fn handle_interrupt(base: usize) {
    let status = read_volatile((base + virtio_mmio::REG_INTERRUPT_STATUS) as *const u32);

    write_volatile(
        (base + virtio_mmio::REG_INTERRUPT_ACK) as *mut u32,
        status & 0x3
    );

    if status & 0x1 != 0 {
        uart::puts("[NET] Used ring updated.\n");
    }
}