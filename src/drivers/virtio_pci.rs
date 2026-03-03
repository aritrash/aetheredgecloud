use core::ptr::{read_volatile, write_volatile};

use crate::drivers::uart;
use crate::drivers::allocator;
use crate::drivers::virtio_queue::{
    VirtqDesc,
    VirtqAvail,
    VirtqUsed,
};

const VIRTIO_STATUS_ACKNOWLEDGE: u8 = 1;
const VIRTIO_STATUS_DRIVER: u8 = 2;
const VIRTIO_STATUS_DRIVER_OK: u8 = 4;
const VIRTIO_STATUS_FEATURES_OK: u8 = 8;
const VIRTIO_STATUS_FAILED: u8 = 0x80;

const VIRTIO_F_VERSION_1: u32 = 1;

#[repr(C)]
pub struct VirtioPciCommonCfg {
    pub device_feature_select: u32,
    pub device_feature: u32,
    pub driver_feature_select: u32,
    pub driver_feature: u32,

    pub msix_config: u16,
    pub num_queues: u16,
    pub device_status: u8,
    pub config_generation: u8,

    pub queue_select: u16,
    pub queue_size: u16,
    pub queue_msix_vector: u16,
    pub queue_enable: u16,
    pub queue_notify_off: u16,

    pub queue_desc_lo: u32,
    pub queue_desc_hi: u32,
    pub queue_avail_lo: u32,
    pub queue_avail_hi: u32,
    pub queue_used_lo: u32,
    pub queue_used_hi: u32,
}

pub struct VirtQueuePci {
    pub desc: *mut VirtqDesc,
    pub avail: *mut VirtqAvail,
    pub used: *mut VirtqUsed,
    pub size: u16,
    pub last_used_idx: u16,
    // Allocator State
    pub free_head: u16,
    pub num_free: u16,
}

impl VirtQueuePci {
    /// Initialize the free list by linking all descriptors together
    pub unsafe fn init_allocator(&mut self) {
        for i in 0..(self.size - 1) {
            (*self.desc.add(i as usize)).next = i + 1;
        }
        // Last one points to a sentinel/null
        (*self.desc.add((self.size - 1) as usize)).next = 0xFFFF; 
        
        self.free_head = 0;
        self.num_free = self.size;
    }

    /// Pull a descriptor from the free list
    pub unsafe fn alloc_desc(&mut self) -> Option<u16> {
        if self.num_free == 0 || self.free_head == 0xFFFF {
            return None;
        }

        let id = self.free_head;
        self.free_head = (*self.desc.add(id as usize)).next;
        self.num_free -= 1;
        
        // Clean the descriptor for fresh use
        let d = self.desc.add(id as usize);
        (*d).addr = 0;
        (*d).len = 0;
        (*d).flags = 0;
        (*d).next = 0;

        Some(id)
    }

    /// Return a descriptor (or a whole chain) to the free list
    pub unsafe fn free_desc_chain(&mut self, mut head: u16) {
        while head != 0xFFFF {
            let d = self.desc.add(head as usize);
            let next = (*d).next;
            let flags = (*d).flags;

            // Link this back into our free list
            (*d).next = self.free_head;
            self.free_head = head;
            self.num_free += 1;

            // If VIRTQ_DESC_F_NEXT (1) isn't set, this was the end of the chain
            if (flags & 1) == 0 {
                break;
            }
            head = next;
        }
    }
}

pub struct VirtioPciTransport {
    pub common_cfg: *mut VirtioPciCommonCfg,
    pub notify_base: usize,
    pub notify_off_multiplier: u32,
}

impl VirtioPciTransport {

    pub unsafe fn new(
        common_cfg: usize,
        notify_base: usize,
        notify_off_multiplier: u32,
    ) -> Self {
        Self {
            common_cfg: common_cfg as *mut _,
            notify_base,
            notify_off_multiplier,
        }
    }

    // ---------------- HANDSHAKE ----------------

    pub unsafe fn handshake(&mut self) -> bool {
        let cfg = &mut *self.common_cfg;

        uart::puts("[VIRTIO] Starting handshake...\n");

        write_volatile(&mut cfg.device_status, 0);

        write_volatile(&mut cfg.device_status, VIRTIO_STATUS_ACKNOWLEDGE);

        write_volatile(
            &mut cfg.device_status,
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER,
        );

        write_volatile(&mut cfg.device_feature_select, 0);
        let device_features = read_volatile(&cfg.device_feature);

        uart::puts("[VIRTIO] Device features: ");
        uart::putc_hex64(device_features as u64);
        uart::puts("\n");

        write_volatile(&mut cfg.driver_feature_select, 0);
        write_volatile(&mut cfg.driver_feature, VIRTIO_F_VERSION_1);

        let mut status = read_volatile(&cfg.device_status);
        status |= VIRTIO_STATUS_FEATURES_OK;
        write_volatile(&mut cfg.device_status, status);

        let verify = read_volatile(&cfg.device_status);
        if (verify & VIRTIO_STATUS_FEATURES_OK) == 0 {
            uart::puts("[VIRTIO] FEATURES_OK rejected\n");
            write_volatile(&mut cfg.device_status, VIRTIO_STATUS_FAILED);
            return false;
        }

        status |= VIRTIO_STATUS_DRIVER_OK;
        write_volatile(&mut cfg.device_status, status);

        let final_status = read_volatile(&cfg.device_status);

        uart::puts("[VIRTIO] Final status: ");
        uart::putc_hex64(final_status as u64);
        uart::puts("\n");

        true
    }

    // ---------------- QUEUE SETUP ----------------

    pub unsafe fn setup_queue(
        &mut self,
        queue_index: u16,
        size: u16,
    ) -> Option<VirtQueuePci> {

        let cfg = &mut *self.common_cfg;

        write_volatile(&mut cfg.queue_select, queue_index);

        let max_size = read_volatile(&cfg.queue_size);
        if max_size == 0 {
            return None;
        }

        let actual_size = if size > max_size { max_size } else { size };
        write_volatile(&mut cfg.queue_size, actual_size);

        let desc_size =
            actual_size as usize * core::mem::size_of::<VirtqDesc>();

        let avail_size =
            core::mem::size_of::<VirtqAvail>();

        let used_size =
            core::mem::size_of::<VirtqUsed>();

        let desc =
            allocator::allocate_aligned(desc_size, 4096) as *mut VirtqDesc;

        let avail =
            allocator::allocate_aligned(avail_size, 4096) as *mut VirtqAvail;

        let used =
            allocator::allocate_aligned(used_size, 4096) as *mut VirtqUsed;

        // Write split 64-bit addresses correctly
        write_volatile(&mut cfg.queue_desc_lo, desc as u32);
        write_volatile(&mut cfg.queue_desc_hi, (desc as u64 >> 32) as u32);

        write_volatile(&mut cfg.queue_avail_lo, avail as u32);
        write_volatile(&mut cfg.queue_avail_hi, (avail as u64 >> 32) as u32);

        write_volatile(&mut cfg.queue_used_lo, used as u32);
        write_volatile(&mut cfg.queue_used_hi, (used as u64 >> 32) as u32);

        write_volatile(&mut cfg.queue_enable, 1);

        uart::puts("[VIRTIO] Queue enabled\n");

        // Note: allocator::allocate_aligned now zeros the memory, 
        // but explicit zeroing here is kept for clarity.
        core::ptr::write_bytes(desc as *mut u8, 0, desc_size);
        core::ptr::write_bytes(avail as *mut u8, 0, avail_size);
        core::ptr::write_bytes(used as *mut u8, 0, used_size);

        let mut vq = VirtQueuePci {
            desc,
            avail,
            used,
            size: actual_size,
            last_used_idx: 0,
            free_head: 0,
            num_free: 0,
        };
        
        vq.init_allocator();
        Some(vq)
    }

    // ---------------- SUBMIT ----------------

    pub unsafe fn submit(&self, queue: &mut VirtQueuePci, head_index: u16) {
        let avail = &mut *queue.avail;
        let slot = avail.idx % queue.size;
        
        avail.ring[slot as usize] = head_index;

        // Ensure descriptor and avail.ring writes are visible before idx update
        core::arch::asm!("dmb ishst", options(nostack));

        avail.idx = avail.idx.wrapping_add(1);

        // Ensure idx update is visible before notify
        core::arch::asm!("dmb ishst", options(nostack));

        self.notify(0); // Notify queue 0
    }

    // ---------------- NOTIFY ----------------

    pub unsafe fn notify(&self, queue_index: u16) {

        let cfg = &mut *self.common_cfg;

        let queue_notify_off =
            read_volatile(&cfg.queue_notify_off);

        let addr =
            self.notify_base +
            (queue_notify_off as usize *
            self.notify_off_multiplier as usize);

        let notify_ptr = addr as *mut u16;

        write_volatile(notify_ptr, queue_index);
    }
}