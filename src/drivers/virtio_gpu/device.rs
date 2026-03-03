use crate::drivers::allocator;
use crate::drivers::uart;
use core::ptr::read_volatile;
use crate::drivers::virtio_pci::{VirtioPciTransport, VirtQueuePci};

use super::commands::*;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 768;
const RESOURCE_ID: u32 = 1;

pub struct VirtioGpu {
    transport: VirtioPciTransport,
    queue: VirtQueuePci,
    framebuffer: *mut u8,
}

impl VirtioGpu {

    pub unsafe fn new(mut transport: VirtioPciTransport) -> Option<Self> {

        if !transport.handshake() {
            return None;
        }

        let queue = transport.setup_queue(0, 128)?;

        let fb_size = (WIDTH * HEIGHT * 4) as usize;
        let framebuffer =
            allocator::allocate_aligned(fb_size, 4096);

        // Fill framebuffer blue (BGRA)
        for i in 0..(WIDTH * HEIGHT) {
            let pixel = framebuffer.add((i * 4) as usize) as *mut u32;
            *pixel = 0x00FF0000;
        }

        // Ensure the blue pixels are actually in RAM before we tell the GPU to "Attach"
        unsafe {
            core::arch::asm!("dmb sy", options(nostack));
        }

        Some(Self {
            transport,
            queue,
            framebuffer,
        })
    }

    unsafe fn submit_cmd<T>(&mut self, cmd: &T) {
        // 1. Allocate Descriptors from our new allocator
        let idx_cmd = self.queue.alloc_desc().expect("GPU Out of Descriptors (Cmd)");
        let idx_resp = self.queue.alloc_desc().expect("GPU Out of Descriptors (Resp)");

        // 2. Setup Command Descriptor (Read-only for device)
        let d_cmd = self.queue.desc.add(idx_cmd as usize);
        (*d_cmd).addr = cmd as *const T as u64;
        (*d_cmd).len = core::mem::size_of::<T>() as u32;
        (*d_cmd).flags = 1; // VIRTQ_DESC_F_NEXT
        (*d_cmd).next = idx_resp;

        // 3. Setup Response Descriptor (Write-only for device)
        // We use a static RESP to avoid stack allocation issues in no_std
        static mut RESP: GpuResp = GpuResp {
            hdr: GpuCtrlHdr { type_: 0, flags: 0, fence_id: 0, ctx_id: 0, padding: 0 }
        };
        
        // FIX: Use addr_of_mut! to get a raw pointer to the packed field
        let resp_type_ptr = core::ptr::addr_of_mut!(RESP.hdr.type_);
        core::ptr::write_volatile(resp_type_ptr, 0);

        let d_resp = self.queue.desc.add(idx_resp as usize);
        (*d_resp).addr = core::ptr::addr_of!(RESP) as u64;
        (*d_resp).len = core::mem::size_of::<GpuResp>() as u32;
        (*d_resp).flags = 2; // VIRTQ_DESC_F_WRITE
        (*d_resp).next = 0;

        // 4. Submit to the Transport
        self.transport.submit(&mut self.queue, idx_cmd);

        // 5. Polling with Volatile Read (Crucial for AArch64)
        // FIX: Use addr_of! for the used ring index as well
        let used_idx_ptr = core::ptr::addr_of!((*self.queue.used).idx);
        while core::ptr::read_volatile(used_idx_ptr) == self.queue.last_used_idx {
            core::hint::spin_loop();
        }

        // 6. Check Response
        // FIX: Use addr_of! to read the response type
        let resp_type = core::ptr::read_volatile(core::ptr::addr_of!(RESP.hdr.type_));
        
        // 7. Housekeeping
        self.queue.last_used_idx = self.queue.last_used_idx.wrapping_add(1);
        
        // IMPORTANT: Return descriptors to the free list for next command!
        self.queue.free_desc_chain(idx_cmd);

        // Check for success: 0x1100 (OK) or 0x1101 (OK_NODATA)
        if resp_type != 0x1100 && resp_type != 0x1101 {
            uart::puts("[GPU] Command Failed! Error: ");
            uart::putc_hex64(resp_type as u64);
            uart::puts("\n");
        }
    }

    pub unsafe fn init_display(&mut self) {
        uart::puts("[GPU] Initializing display...\n");

        // ---------------------------------------------------
        // 1. GET_DISPLAY_INFO
        // ---------------------------------------------------
        let get_info = GetDisplayInfo {
            hdr: GpuCtrlHdr {
                type_: VIRTIO_GPU_CMD_GET_DISPLAY_INFO,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                padding: 0,
            }
        };
        self.submit_cmd(&get_info);

        // ---------------------------------------------------
        // 2. RESOURCE_CREATE_2D
        // ---------------------------------------------------
        let create = ResourceCreate2D {
            hdr: GpuCtrlHdr {
                type_: VIRTIO_GPU_CMD_RESOURCE_CREATE_2D,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                padding: 0,
            },
            resource_id: RESOURCE_ID,
            format: VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM,
            width: WIDTH,
            height: HEIGHT,
        };
        self.submit_cmd(&create);

        uart::puts("[GPU] FB addr: ");
        uart::putc_hex64(self.framebuffer as u64);
        uart::puts("\n");

        // ---------------------------------------------------
        // 3. RESOURCE_ATTACH_BACKING
        // ---------------------------------------------------
        // CRITICAL: This MUST be packed to match VirtIO spec exactly.
        #[repr(C, packed)]
        struct AttachBackingFull {
            attach: ResourceAttachBacking,
            entry: MemEntry,
        }

        let full = AttachBackingFull {
            attach: ResourceAttachBacking {
                hdr: GpuCtrlHdr {
                    type_: VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING,
                    flags: 0,
                    fence_id: 0,
                    ctx_id: 0,
                    padding: 0,
                },
                resource_id: RESOURCE_ID,
                nr_entries: 1,
            },
            entry: MemEntry {
                addr: self.framebuffer as u64,
                length: WIDTH * HEIGHT * 4,
                padding: 0,
            },
        };
        self.submit_cmd(&full);

        // ---------------------------------------------------
        // 4. SET_SCANOUT
        // ---------------------------------------------------
        let scanout = SetScanout {
            hdr: GpuCtrlHdr {
                type_: VIRTIO_GPU_CMD_SET_SCANOUT,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                padding: 0,
            },
            rect: Rect {
                x: 0,
                y: 0,
                width: WIDTH,
                height: HEIGHT,
            },
            scanout_id: 0,
            resource_id: RESOURCE_ID,
        };
        self.submit_cmd(&scanout);

        // ---------------------------------------------------
        // 5. TRANSFER_TO_HOST_2D
        // ---------------------------------------------------
        let transfer = TransferToHost2D {
            hdr: GpuCtrlHdr {
                type_: VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                padding: 0,
            },
            rect: Rect {
                x: 0,
                y: 0,
                width: WIDTH,
                height: HEIGHT,
            },
            offset: 0,
            resource_id: RESOURCE_ID,
            padding: 0,
        };
        self.submit_cmd(&transfer);

        // ---------------------------------------------------
        // 6. RESOURCE_FLUSH
        // ---------------------------------------------------
        let flush = ResourceFlush {
            hdr: GpuCtrlHdr {
                type_: VIRTIO_GPU_CMD_RESOURCE_FLUSH,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                padding: 0,
            },
            rect: Rect {
                x: 0,
                y: 0,
                width: WIDTH,
                height: HEIGHT,
            },
            resource_id: RESOURCE_ID,
            padding: 0,
        };
        self.submit_cmd(&flush);

        uart::puts("[GPU] Initialization complete. Check QEMU window!\n");
    }
}