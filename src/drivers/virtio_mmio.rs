use core::ptr::{read_volatile, write_volatile};
use crate::drivers::uart;
use crate::drivers::virtio_queue::{VirtQueue, VirtqDesc, VirtqAvail, VirtqUsed};
use core::sync::atomic::{self, Ordering};

//
// =======================
//  MMIO REGISTER OFFSETS
// =======================
//

pub const REG_MAGIC:           usize = 0x000;
pub const REG_VERSION:         usize = 0x004;
pub const REG_DEVICE_ID:       usize = 0x008;
pub const REG_VENDOR_ID:       usize = 0x00c;
pub const REG_DEVICE_FEATURES: usize = 0x010;
pub const REG_DRIVER_FEATURES: usize = 0x020;
pub const REG_QUEUE_SEL:       usize = 0x030;
pub const REG_QUEUE_NUM_MAX:   usize = 0x034;
pub const REG_QUEUE_NUM:       usize = 0x038;
pub const REG_QUEUE_READY:     usize = 0x044;
pub const REG_QUEUE_NOTIFY:    usize = 0x050;
pub const REG_INTERRUPT_STATUS:usize = 0x060;
pub const REG_INTERRUPT_ACK:   usize = 0x064;
pub const REG_STATUS:          usize = 0x070;

pub const REG_QUEUE_DESC_LOW:   usize = 0x080;
pub const REG_QUEUE_DESC_HIGH:  usize = 0x084;
pub const REG_QUEUE_AVAIL_LOW:  usize = 0x090;
pub const REG_QUEUE_AVAIL_HIGH: usize = 0x094;
pub const REG_QUEUE_USED_LOW:   usize = 0x0a0;
pub const REG_QUEUE_USED_HIGH:  usize = 0x0a4;

//
// =======================
//  STATUS FLAGS
// =======================
//

pub const STATUS_ACKNOWLEDGE: u32 = 1;
pub const STATUS_DRIVER:      u32 = 2;
pub const STATUS_DRIVER_OK:   u32 = 4;
pub const STATUS_FEATURES_OK: u32 = 8;

//
// =======================
//  GENERIC PROBE
// =======================
//

pub unsafe fn probe(base: usize, expected_device_id: u32) -> bool {
    let magic = read_volatile(base as *const u32);
    if magic != 0x74726976 { // "virt"
        return false;
    }

    let device_id = read_volatile((base + REG_DEVICE_ID) as *const u32);
    device_id == expected_device_id
}

//
// =======================
//  GENERIC HANDSHAKE
// =======================
//

pub unsafe fn init_device(base: usize) {
    uart::puts("[VirtIO] Initializing device...\n");

    // 1. Reset
    write_volatile((base + REG_STATUS) as *mut u32, 0);

    // 2. ACK + DRIVER
    let mut status = STATUS_ACKNOWLEDGE | STATUS_DRIVER;
    write_volatile((base + REG_STATUS) as *mut u32, status);

    // 3. Feature negotiation (accept all for now)
    let features = read_volatile((base + REG_DEVICE_FEATURES) as *const u32);
    write_volatile((base + REG_DRIVER_FEATURES) as *mut u32, features);

    // 4. FEATURES_OK
    status |= STATUS_FEATURES_OK;
    write_volatile((base + REG_STATUS) as *mut u32, status);

    // 5. Verify FEATURES_OK
    let check = read_volatile((base + REG_STATUS) as *const u32);
    if (check & STATUS_FEATURES_OK) == 0 {
        uart::puts("[FATAL] VirtIO device rejected features.\n");
        loop {}
    }

    // 6. DRIVER_OK
    status |= STATUS_DRIVER_OK;
    write_volatile((base + REG_STATUS) as *mut u32, status);

    let final_status = read_volatile((base + REG_STATUS) as *const u32);
    uart::puts("[VirtIO] Device Ready. Status: ");
    uart::putc_hex64(final_status as u64);
    uart::puts("\n");
}

//
// =======================
//  GENERIC QUEUE SETUP
// =======================
//

pub unsafe fn setup_queue(base: usize, q_idx: u16) -> VirtQueue {
    write_volatile((base + REG_QUEUE_SEL) as *mut u32, q_idx as u32);

    let q_max = read_volatile((base + REG_QUEUE_NUM_MAX) as *const u32);
    let q_size = if q_max < 128 { q_max } else { 128 };

    write_volatile((base + REG_QUEUE_NUM) as *mut u32, q_size);

    // ----------------------------
    // Descriptor Table
    // ----------------------------
    let desc_size = q_size as usize * core::mem::size_of::<VirtqDesc>();

    let desc = crate::drivers::allocator::allocate_aligned(
        desc_size,
        16,
    ) as *mut VirtqDesc;

    core::ptr::write_bytes(desc as *mut u8, 0, desc_size);

    // ----------------------------
    // Available Ring
    // Layout:
    // flags (2)
    // idx   (2)
    // ring  (2 * q_size)
    // used_event (2)
    // ----------------------------
    let avail_size = 4 + (q_size as usize * 2) + 2;

    let avail = crate::drivers::allocator::allocate_aligned(
        avail_size,
        2,
    ) as *mut VirtqAvail;

    core::ptr::write_bytes(avail as *mut u8, 0, avail_size);

    // ----------------------------
    // Used Ring
    // Layout:
    // flags (2)
    // idx   (2)
    // ring  (8 * q_size)
    // avail_event (2)
    // ----------------------------
    let used_size = 4 + (q_size as usize * 8) + 2;

    let used = crate::drivers::allocator::allocate_aligned(
        used_size,
        4,
    ) as *mut VirtqUsed;

    core::ptr::write_bytes(used as *mut u8, 0, used_size);

    // ----------------------------
    // Register physical addresses
    // ----------------------------
    write_volatile((base + REG_QUEUE_DESC_LOW) as *mut u32, desc as usize as u32);
    write_volatile((base + REG_QUEUE_DESC_HIGH) as *mut u32, ((desc as usize as u64) >> 32) as u32);

    write_volatile((base + REG_QUEUE_AVAIL_LOW) as *mut u32, avail as usize as u32);
    write_volatile((base + REG_QUEUE_AVAIL_HIGH) as *mut u32, ((avail as usize as u64) >> 32) as u32);

    write_volatile((base + REG_QUEUE_USED_LOW) as *mut u32, used as usize as u32);
    write_volatile((base + REG_QUEUE_USED_HIGH) as *mut u32, ((used as usize as u64) >> 32) as u32);

    write_volatile((base + REG_QUEUE_READY) as *mut u32, 1);

    uart::puts("[VirtIO] Queue configured.\n");

    VirtQueue {
        desc,
        avail,
        used,
        queue_idx: q_idx,
        last_used_idx: 0,
        size: q_size as u16,
    }
}

//
// =======================
//  QUEUE NOTIFY
// =======================
//

pub unsafe fn notify_queue(base: usize, queue_index: u16) {
    atomic::fence(Ordering::SeqCst);
    write_volatile((base + REG_QUEUE_NOTIFY) as *mut u32, queue_index as u32);
}