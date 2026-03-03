#![allow(dead_code)]

pub const VIRTIO_GPU_CMD_RESOURCE_CREATE_2D: u32 = 0x0100;
pub const VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING: u32 = 0x0106;
pub const VIRTIO_GPU_CMD_SET_SCANOUT: u32 = 0x0103;
pub const VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D: u32 = 0x0105;
pub const VIRTIO_GPU_CMD_RESOURCE_FLUSH: u32 = 0x0104;

pub const VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM: u32 = 1;
pub const VIRTIO_GPU_CMD_GET_DISPLAY_INFO: u32 = 0x0101;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GpuCtrlHdr {
    pub type_: u32,
    pub flags: u32,
    pub fence_id: u64,
    pub ctx_id: u32,
    pub padding: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C, packed)]
pub struct ResourceCreate2D {
    pub hdr: GpuCtrlHdr,
    pub resource_id: u32,
    pub format: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C, packed)]
pub struct MemEntry {
    pub addr: u64,
    pub length: u32,
    pub padding: u32,
}

#[repr(C, packed)]
pub struct ResourceAttachBacking {
    pub hdr: GpuCtrlHdr,
    pub resource_id: u32,
    pub nr_entries: u32,
}

#[repr(C, packed)]
pub struct SetScanout {
    pub hdr: GpuCtrlHdr,
    pub rect: Rect,
    pub scanout_id: u32,
    pub resource_id: u32,
}

#[repr(C, packed)]
pub struct TransferToHost2D {
    pub hdr: GpuCtrlHdr,
    pub rect: Rect,
    pub offset: u64,
    pub resource_id: u32,
    pub padding: u32,
}

#[repr(C, packed)]
pub struct ResourceFlush {
    pub hdr: GpuCtrlHdr,
    pub rect: Rect,
    pub resource_id: u32,
    pub padding: u32,
}

#[repr(C, packed)]
pub struct GpuResp {
    pub hdr: GpuCtrlHdr,
}

#[repr(C, packed)]
pub struct GetDisplayInfo {
    pub hdr: GpuCtrlHdr,
}