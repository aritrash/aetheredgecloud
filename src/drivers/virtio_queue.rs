pub struct VirtQueue {
    pub desc: *mut VirtqDesc,
    pub avail: *mut VirtqAvail,
    pub used: *mut VirtqUsed,
    pub queue_idx: u16,
    pub last_used_idx: u16,
    pub size: u16, // Added this
}

#[repr(C, align(16))]
#[derive(Copy, Clone)]
pub struct VirtqDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

#[repr(C)]
pub struct VirtqAvail {
    pub flags: u16,
    pub idx: u16,
    pub ring: [u16; 128],
    pub used_event: u16,
}

#[repr(C)]
pub struct VirtqUsed {
    pub flags: u16,
    pub idx: u16,
    pub ring: [VirtqUsedElem; 128],
    pub avail_event: u16,
}

#[repr(C, align(4))]
#[derive(Copy, Clone)]
pub struct VirtqUsedElem {
    pub id: u32,
    pub len: u32,
}

impl VirtQueue {
    pub unsafe fn add_desc(&mut self, i: u16, addr: u64, len: u32, flags: u16) {
        let d = self.desc.add(i as usize);
        (*d).addr = addr;
        (*d).len = len;
        (*d).flags = flags;
        (*d).next = 0;
    }
}