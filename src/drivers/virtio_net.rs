use core::ptr::{read_volatile, write_volatile};
use crate::drivers::uart;
use crate::drivers::virtio_queue::{VirtQueue, VirtqDesc, VirtqAvail, VirtqUsed};
use core::sync::atomic::{self, Ordering};

// --- MMIO Offsets ---
const REG_MAGIC:           usize = 0x000;
const REG_VERSION:         usize = 0x004;
const REG_DEVICE_ID:       usize = 0x008;
const REG_VENDOR_ID:       usize = 0x00c;
const REG_DEVICE_FEATURES: usize = 0x010;
const REG_DRIVER_FEATURES: usize = 0x020;
const REG_QUEUE_SEL:       usize = 0x030;
const REG_QUEUE_NUM_MAX:   usize = 0x034;
const REG_QUEUE_NUM:       usize = 0x038;
const REG_QUEUE_READY:     usize = 0x044;
const REG_QUEUE_NOTIFY:    usize = 0x050;
const REG_INTERRUPT_STATUS:usize = 0x060;
const REG_INTERRUPT_ACK:   usize = 0x064;
const REG_STATUS:          usize = 0x070;

// --- Queue Address Registers ---
const REG_QUEUE_DESC_LOW:   usize = 0x080;
const REG_QUEUE_DESC_HIGH:  usize = 0x084;
const REG_QUEUE_AVAIL_LOW:  usize = 0x090;
const REG_QUEUE_AVAIL_HIGH: usize = 0x094;
const REG_QUEUE_USED_LOW:   usize = 0x0a0;
const REG_QUEUE_USED_HIGH:  usize = 0x0a4;

// --- Status Bits ---
const STATUS_ACKNOWLEDGE: u32 = 1;
const STATUS_DRIVER:      u32 = 2;
const STATUS_FEATURES_OK: u32 = 8;
const STATUS_DRIVER_OK:   u32 = 4; 

pub unsafe fn init(base: usize) -> (VirtQueue, VirtQueue) {
    uart::puts("[VirtIO] Initializing NIC Handshake...\n");

    // 1. Reset device
    write_volatile((base + REG_STATUS) as *mut u32, 0);

    // 2. ACK + DRIVER
    let mut status = STATUS_ACKNOWLEDGE | STATUS_DRIVER;
    write_volatile((base + REG_STATUS) as *mut u32, status);

    // 3. Negotiate features (accept all for now)
    let features = read_volatile((base + REG_DEVICE_FEATURES) as *const u32);
    write_volatile((base + REG_DRIVER_FEATURES) as *mut u32, features);

    // 4. FEATURES_OK
    status |= STATUS_FEATURES_OK;
    write_volatile((base + REG_STATUS) as *mut u32, status);

    // 5. Verify FEATURES_OK stuck
    let check_status = read_volatile((base + REG_STATUS) as *const u32);
    if (check_status & STATUS_FEATURES_OK) == 0 {
        uart::puts("[FATAL] VirtIO NIC rejected features.\n");
        loop {}
    }

    // 6. Setup queues
    let mut rx_q = setup_queue(base, 0);
    let tx_q = setup_queue(base, 1);

    prime_rx_queue(base, &mut rx_q);

    // 7. DRIVER_OK
    status |= STATUS_DRIVER_OK;
    write_volatile((base + REG_STATUS) as *mut u32, status);

    let final_status = read_volatile((base + REG_STATUS) as *const u32);
    uart::puts("[VirtIO] Final Device Status: ");
    uart::putc_hex64(final_status as u64);
    uart::puts("\n");

    (rx_q, tx_q)
}

pub unsafe fn setup_queue(base: usize, q_idx: u16) -> VirtQueue {
    write_volatile((base + REG_QUEUE_SEL) as *mut u32, q_idx as u32);

    let q_max = read_volatile((base + REG_QUEUE_NUM_MAX) as *const u32);
    let q_size = if q_max < 128 { q_max } else { 128 };
    write_volatile((base + REG_QUEUE_NUM) as *mut u32, q_size);

    let desc = crate::drivers::allocator::allocate_aligned(q_size as usize * 16, 16) as *mut VirtqDesc;
    let avail = crate::drivers::allocator::allocate_aligned(6 + (q_size as usize * 2), 2) as *mut VirtqAvail;
    let used = crate::drivers::allocator::allocate_aligned(6 + (q_size as usize * 8), 4) as *mut VirtqUsed;

    // Properly zero full descriptor area
    core::ptr::write_bytes(desc, 0, (q_size as usize) * core::mem::size_of::<VirtqDesc>());
    core::ptr::write_bytes(avail, 0, 6 + (q_size as usize * 2));
    core::ptr::write_bytes(used, 0, 6 + (q_size as usize * 8));

    write_volatile((base + REG_QUEUE_DESC_LOW) as *mut u32, desc as usize as u32);
    write_volatile((base + REG_QUEUE_DESC_HIGH) as *mut u32, ((desc as usize as u64) >> 32) as u32);

    write_volatile((base + REG_QUEUE_AVAIL_LOW) as *mut u32, avail as usize as u32);
    write_volatile((base + REG_QUEUE_AVAIL_HIGH) as *mut u32, ((avail as usize as u64) >> 32) as u32);

    write_volatile((base + REG_QUEUE_USED_LOW) as *mut u32, used as usize as u32);
    write_volatile((base + REG_QUEUE_USED_HIGH) as *mut u32, ((used as usize as u64) >> 32) as u32);

    write_volatile((base + REG_QUEUE_READY) as *mut u32, 1);

    uart::puts("  [OK] Configured Queue ");
    uart::putc(b'0' + q_idx as u8);
    uart::puts("\n");

    VirtQueue { 
        desc, avail, used, 
        queue_idx: q_idx, 
        last_used_idx: 0,
        size: q_size as u16,
    }
}

pub unsafe fn probe(base: usize) -> bool {
    let magic = read_volatile(base as *const u32);
    if magic != 0x74726976 { return false; }
    let device_id = read_volatile((base + REG_DEVICE_ID) as *const u32);
    device_id == 1
}

pub unsafe fn prime_rx_queue(base: usize, rx_q: &mut VirtQueue) {
    for i in 0..rx_q.size {
        let buf = crate::drivers::allocator::allocate_aligned(1536, 16);
        rx_q.add_desc(i, buf as u64, 1536, 2);
        (*rx_q.avail).ring[i as usize] = i;
    }

    atomic::fence(Ordering::SeqCst);
    (*rx_q.avail).idx = rx_q.size;
    atomic::fence(Ordering::SeqCst);

    write_volatile((base + REG_QUEUE_NOTIFY) as *mut u32, 0);
}

pub unsafe fn check_device_status(base: usize) -> u32 {
    read_volatile((base + REG_STATUS) as *const u32)
}

pub unsafe fn is_device_ready(base: usize) -> bool {
    let status = check_device_status(base);
    (status & 0xF) == 0xF
}

#[no_mangle]
pub unsafe fn handle_interrupt() {
    let base = 0x0A003E00;

    let status = read_volatile((base + REG_INTERRUPT_STATUS) as *const u32);
    write_volatile((base + REG_INTERRUPT_ACK) as *mut u32, status & 0x3);

    if status & 0x1 != 0 {
        uart::puts("[NET] Used Ring Updated (Network Activity)\n");
    }
}