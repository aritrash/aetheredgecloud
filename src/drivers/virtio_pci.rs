use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{fence, Ordering};

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

/// Modern VirtIO PCI Common Configuration structure
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

/// VirtIO PCI transport abstraction
pub struct VirtioPciTransport {
    pub common_cfg: *mut VirtioPciCommonCfg,
    pub notify_base: usize,
}

impl VirtioPciTransport {

    /// Create transport from discovered addresses
    pub unsafe fn new(common_cfg: usize, notify_base: usize) -> Self {
        Self {
            common_cfg: common_cfg as *mut VirtioPciCommonCfg,
            notify_base,
        }
    }

    /// Perform full spec-compliant handshake
    pub unsafe fn handshake(&mut self) -> bool {
        let cfg = &mut *self.common_cfg;

        uart::puts("[VIRTIO] Starting handshake...\n");

        // Reset
        write_volatile(&mut cfg.device_status, 0);

        // ACKNOWLEDGE
        write_volatile(&mut cfg.device_status, VIRTIO_STATUS_ACKNOWLEDGE);

        // DRIVER
        write_volatile(
            &mut cfg.device_status,
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER,
        );

        // Read device features (page 0)
        write_volatile(&mut cfg.device_feature_select, 0);
        let device_features = read_volatile(&cfg.device_feature);

        uart::puts("[VIRTIO] Device features: ");
        uart::putc_hex64(device_features as u64);
        uart::puts("\n");

        // Negotiate: accept only VERSION_1
        write_volatile(&mut cfg.driver_feature_select, 0);
        write_volatile(&mut cfg.driver_feature, VIRTIO_F_VERSION_1);

        // FEATURES_OK
        let mut status = read_volatile(&cfg.device_status);
        status |= VIRTIO_STATUS_FEATURES_OK;
        write_volatile(&mut cfg.device_status, status);

        // Verify FEATURES_OK sticks
        let verify = read_volatile(&cfg.device_status);
        if (verify & VIRTIO_STATUS_FEATURES_OK) == 0 {
            uart::puts("[VIRTIO] FEATURES_OK rejected\n");
            write_volatile(&mut cfg.device_status, VIRTIO_STATUS_FAILED);
            return false;
        }

        // DRIVER_OK
        status |= VIRTIO_STATUS_DRIVER_OK;
        write_volatile(&mut cfg.device_status, status);

        let final_status = read_volatile(&cfg.device_status);

        uart::puts("[VIRTIO] Final status: ");
        uart::putc_hex64(final_status as u64);
        uart::puts("\n");

        true
    }

    /// Setup a queue and return ring memory pointers
    pub unsafe fn setup_queue(
        &mut self,
        queue_index: u16,
        max_size: u16,
    ) -> Option<(u16, *mut VirtqDesc, *mut VirtqAvail, *mut VirtqUsed)> {

        let cfg = &mut *self.common_cfg;

        uart::puts("[VIRTIO] Setting up queue ");
        uart::putc_hex64(queue_index as u64);
        uart::puts("...\n");

        // Select queue
        write_volatile(&mut cfg.queue_select, queue_index);

        let device_q_size = read_volatile(&cfg.queue_size);

        if device_q_size == 0 {
            uart::puts("[VIRTIO] Queue not available\n");
            return None;
        }

        let q_size = if device_q_size > max_size {
            max_size
        } else {
            device_q_size
        };

        uart::puts("[VIRTIO] Queue size: ");
        uart::putc_hex64(q_size as u64);
        uart::puts("\n");

        write_volatile(&mut cfg.queue_size, q_size);

        // Allocate rings
        let desc_size =
            q_size as usize * core::mem::size_of::<VirtqDesc>();
        let avail_size =
            4 + (q_size as usize * 2) + 2;
        let used_size =
            4 + (q_size as usize * 8) + 2;

        let desc =
            allocator::allocate_aligned(desc_size, 16) as *mut VirtqDesc;
        let avail =
            allocator::allocate_aligned(avail_size, 2) as *mut VirtqAvail;
        let used =
            allocator::allocate_aligned(used_size, 4) as *mut VirtqUsed;

        core::ptr::write_bytes(desc as *mut u8, 0, desc_size);
        core::ptr::write_bytes(avail as *mut u8, 0, avail_size);
        core::ptr::write_bytes(used as *mut u8, 0, used_size);

        let desc_addr = desc as u64;
        let avail_addr = avail as u64;
        let used_addr = used as u64;

        write_volatile(&mut cfg.queue_desc_lo, desc_addr as u32);
        write_volatile(&mut cfg.queue_desc_hi, (desc_addr >> 32) as u32);

        write_volatile(&mut cfg.queue_avail_lo, avail_addr as u32);
        write_volatile(&mut cfg.queue_avail_hi, (avail_addr >> 32) as u32);

        write_volatile(&mut cfg.queue_used_lo, used_addr as u32);
        write_volatile(&mut cfg.queue_used_hi, (used_addr >> 32) as u32);

        // Enable queue
        write_volatile(&mut cfg.queue_enable, 1);

        uart::puts("[VIRTIO] Queue enabled\n");

        Some((q_size, desc, avail, used))
    }

    /// Notify device about queue activity
    pub unsafe fn notify(&self, queue_index: u16) {
        fence(Ordering::SeqCst);

        let notify_ptr = (self.notify_base as *mut u16)
            .add(queue_index as usize);

        write_volatile(notify_ptr, queue_index);
    }
}