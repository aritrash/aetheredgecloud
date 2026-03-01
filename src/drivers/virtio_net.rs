use core::ptr::{read_volatile, write_volatile};
use crate::drivers::uart;
use crate::drivers::virtio_queue::{VirtQueue, VirtqDesc, VirtqAvail, VirtqUsed};

// MMIO Offsets
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
const REG_STATUS:          usize = 0x070;

// Queue Address Registers (64-bit split into two 32-bit registers)
const REG_QUEUE_DESC_LOW:  usize = 0x080;
const REG_QUEUE_DESC_HIGH: usize = 0x084;
const REG_QUEUE_AVAIL_LOW: usize = 0x090;
const REG_QUEUE_AVAIL_HIGH:usize = 0x094;
const REG_QUEUE_USED_LOW:  usize = 0x0a0;
const REG_QUEUE_USED_HIGH: usize = 0x0a4;

// Status Bits
const STATUS_ACKNOWLEDGE: u32 = 1;
const STATUS_DRIVER:      u32 = 2;
const STATUS_FEATURES_OK: u32 = 8;
const STATUS_DRIVER_OK:   u32 = 128;

pub unsafe fn init(base: usize) -> (VirtQueue, VirtQueue) {
    uart::puts("[VirtIO] Initializing NIC Handshake...\n");

    // 1. Reset device
    write_volatile((base + REG_STATUS) as *mut u32, 0);

    // 2. Set ACKNOWLEDGE bit
    let mut status = STATUS_ACKNOWLEDGE;
    write_volatile((base + REG_STATUS) as *mut u32, status);

    // 3. Set DRIVER bit
    status |= STATUS_DRIVER;
    write_volatile((base + REG_STATUS) as *mut u32, status);

    // 4. Feature Negotiation
    let features = read_volatile((base + REG_DEVICE_FEATURES) as *const u32);
    uart::puts("  [INFO] Negotiating features: ");
    uart::putc_hex64(features as u64);
    uart::puts("\n");
    
    write_volatile((base + REG_DRIVER_FEATURES) as *mut u32, features);

    // 5. Transition to FEATURES_OK
    status |= STATUS_FEATURES_OK;
    write_volatile((base + REG_STATUS) as *mut u32, status);

    // Verification
    if read_volatile((base + REG_STATUS) as *const u32) & STATUS_FEATURES_OK == 0 {
        uart::puts("[FATAL] VirtIO NIC refused feature set.\n");
        // In a real kernel we'd panic here, for now we hang to avoid returning junk
        loop {} 
    }

    // 6. Setup VirtQueues
    let mut rx_q = setup_queue(base, 0);
    let tx_q = setup_queue(base, 1);

    // Provide buffers to the device
    prime_rx_queue(base, &mut rx_q);

    // Finalize: DRIVER_OK
    let mut status = read_volatile((base + REG_STATUS) as *const u32);
    status |= STATUS_DRIVER_OK;
    write_volatile((base + REG_STATUS) as *mut u32, status);

    uart::puts("[OK] VirtIO NIC is now ACTIVE and LISTENING.\n");

    (rx_q, tx_q) // Return the initialized queues
}

pub unsafe fn setup_queue(base: usize, q_idx: u16) -> VirtQueue {
    // Select the queue we want to configure
    write_volatile((base + REG_QUEUE_SEL) as *mut u32, q_idx as u32);

    // Check maximum queue size supported by device
    let q_max = read_volatile((base + REG_QUEUE_NUM_MAX) as *const u32);
    let q_size = if q_max < 128 { q_max } else { 128 };

    // Set the queue size we will use
    write_volatile((base + REG_QUEUE_NUM) as *mut u32, q_size);

    // Allocate aligned memory for the three parts of the queue
    // Descriptor Table: q_size * 16 bytes (aligned to 16)
    let desc = crate::drivers::allocator::allocate_aligned(q_size as usize * 16, 16) as *mut VirtqDesc;
    
    // Available Ring: 6 + (q_size * 2) bytes (aligned to 2)
    let avail = crate::drivers::allocator::allocate_aligned(6 + (q_size as usize * 2), 2) as *mut VirtqAvail;
    
    // Used Ring: 6 + (q_size * 8) bytes (aligned to 4)
    let used = crate::drivers::allocator::allocate_aligned(6 + (q_size as usize * 8), 4) as *mut VirtqUsed;

    // Zero out the allocated memory (Optional but recommended for safety)
    core::ptr::write_bytes(desc, 0, q_size as usize);
    core::ptr::write_bytes(avail, 0, 1);
    core::ptr::write_bytes(used, 0, 1);

    // Provide physical addresses to the device (Identity mapped: VA == PA)
    write_volatile((base + REG_QUEUE_DESC_LOW) as *mut u32, desc as u32);
    write_volatile((base + REG_QUEUE_DESC_HIGH) as *mut u32, ((desc as u64) >> 32) as u32);

    write_volatile((base + REG_QUEUE_AVAIL_LOW) as *mut u32, avail as u32);
    write_volatile((base + REG_QUEUE_AVAIL_HIGH) as *mut u32, ((avail as u64) >> 32) as u32);

    write_volatile((base + REG_QUEUE_USED_LOW) as *mut u32, used as u32);
    write_volatile((base + REG_QUEUE_USED_HIGH) as *mut u32, ((used as u64) >> 32) as u32);

    // Tell device this queue is ready to be used
    write_volatile((base + REG_QUEUE_READY) as *mut u32, 1);

    uart::puts("  [OK] Configured Queue ");
    uart::putc(b'0' + q_idx as u8);
    uart::puts("\n");

    VirtQueue { 
        desc, 
        avail, 
        used, 
        queue_idx: q_idx, 
        last_used_idx: 0,
        size: q_size as u16,
    }
}

pub unsafe fn probe(base: usize) -> bool {
    let magic = read_volatile(base as *const u32);
    if magic != 0x74726976 { return false; }

    let device_id = read_volatile((base + REG_DEVICE_ID) as *const u32);
    
    uart::puts("[VirtIO] Found device at ");
    uart::putc_hex64(base as u64);

    if device_id == 1 {
        uart::puts(" -> [NETWORK CARD]\n");
        return true;
    } else {
        uart::puts(" (ID: ");
        uart::putc_hex64(device_id as u64);
        uart::puts(") -> [SKIPPED]\n");
        return false;
    }
}

// In src/drivers/virtio_net.rs

const VIRTQ_DESC_F_WRITE: u16 = 2; // Device can write to this buffer

pub unsafe fn prime_rx_queue(base: usize, rx_q: &mut VirtQueue) {
    uart::puts("[VirtIO] Priming RX Queue with buffers...\n");

    for i in 0..rx_q.size {
        // 1. Allocate a buffer for one Ethernet frame
        // 1514 (Max Eth) + 12 (VirtIO Header) = 1526. Let's round to 1536 for alignment.
        let buf = crate::drivers::allocator::allocate_aligned(1536, 16);
        
        // 2. Add to Descriptor Table
        rx_q.add_desc(i, buf as u64, 1536, VIRTQ_DESC_F_WRITE);
        
        // 3. Put the descriptor index into the Available Ring
        (*rx_q.avail).ring[i as usize] = i;
    }

    // 4. Update the Available Index to show 128 buffers are ready
    (*rx_q.avail).idx = rx_q.size;

    // 5. Notify the device (Writing the Queue Index to the Queue Notify register)
    // On MMIO, the notify register is usually at offset 0x050
    write_volatile((base + 0x050) as *mut u32, 0); // 0 = RX Queue

    uart::puts("[OK] RX Queue primed and Device notified.\n");
}

// MMIO Offset for Interrupt Status and Acknowledge
const REG_INTERRUPT_STATUS: usize = 0x060;
const REG_INTERRUPT_ACK:    usize = 0x064;

#[no_mangle]
pub unsafe fn handle_interrupt() {
    // 1. We need to know which VirtIO device triggered this.
    // For now, we'll assume it's our NIC at 0x0A003E00. 
    // In a real OS, we'd look up the base address from an IRQ mapping table.
    let base = 0x0A003E00;

    // 2. Read the interrupt status
    let status = read_volatile((base + REG_INTERRUPT_STATUS) as *const u32);

    // 3. Acknowledge the interrupt (Clear it)
    write_volatile((base + REG_INTERRUPT_ACK) as *mut u32, status & 0x3);

    // Bit 0 = Used Ring Update (packet received or sent)
    // Bit 1 = Configuration Change
    if status & 0x1 != 0 {
        uart::puts("[NET] Used Ring Updated (Packet activity detected!)\n");
        
        // This is where we will call: 
        // process_used_ring(base);
    }

    if status & 0x2 != 0 {
        uart::puts("[NET] Configuration changed.\n");
    }
}