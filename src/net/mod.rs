use crate::drivers::virtio_queue::VirtQueue;
use smoltcp::phy::{self, DeviceCapabilities, Medium};
use smoltcp::time::Instant;

// --- 1. Struct Definitions ---

pub struct VirtioNetDevice {
    pub base: usize,
    pub rx: VirtQueue,
    pub tx: VirtQueue,
}

pub struct VirtioRxToken<'a> {
    pub queue: &'a mut VirtQueue,
}

pub struct VirtioTxToken<'a> {
    pub queue: &'a mut VirtQueue,
    pub base: usize,
}

// --- 2. Device Trait Implementation ---

impl smoltcp::phy::Device for VirtioNetDevice {
    type RxToken<'a> = VirtioRxToken<'a> where Self: 'a;
    type TxToken<'a> = VirtioTxToken<'a> where Self: 'a;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        unsafe {
            let used_idx = (*self.rx.used).idx;
            if self.rx.last_used_idx != used_idx {
                return Some((
                    VirtioRxToken { queue: &mut self.rx },
                    VirtioTxToken { queue: &mut self.tx, base: self.base }
                ));
            }
        }
        None
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(VirtioTxToken { queue: &mut self.tx, base: self.base })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1514;
        caps.medium = Medium::Ethernet;
        caps
    }
}

// --- 3. Token Implementations ---

impl<'a> phy::RxToken for VirtioRxToken<'a> {
    // Note: No 'b, No extra lifetimes. Just R and F.
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R, // MUST BE MUTABLE
    {
        unsafe {
            let last = self.queue.last_used_idx % self.queue.size;
            let used_elem = &(*self.queue.used).ring[last as usize];
            
            let desc_ptr = self.queue.desc.add(used_elem.id as usize);
            
            // Note: We cast to *mut u8 because the trait wants &mut [u8]
            let data_ptr = ((*desc_ptr).addr + 12) as *mut u8;
            let data_len = (used_elem.len - 12) as usize;
            
            let slice = core::slice::from_raw_parts_mut(data_ptr, data_len);

            let result = f(slice);

            // Update hardware indices
            self.queue.last_used_idx = self.queue.last_used_idx.wrapping_add(1);
            (*self.queue.avail).idx = (*self.queue.avail).idx.wrapping_add(1);
            
            result
        }
    }
}

impl<'a> phy::TxToken for VirtioTxToken<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        unsafe {
            let desc_ptr = self.queue.desc.add(0);
            let data_ptr = ((*desc_ptr).addr + 12) as *mut u8;
            let slice = core::slice::from_raw_parts_mut(data_ptr, len);

            let result = f(slice);

            (*self.queue.avail).ring[0] = 0;
            (*self.queue.avail).idx = (*self.queue.avail).idx.wrapping_add(1);
            
            core::ptr::write_volatile((self.base + 0x050) as *mut u32, 1);

            result
        }
    }
}