use crate::drivers::virtio_queue::VirtQueue;
use smoltcp::phy::{self, DeviceCapabilities, Medium};
use smoltcp::time::Instant;
use core::sync::atomic::{self, Ordering};

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
        // If this doesn't print, smoltcp is blocking the transmit internally
        // crate::drivers::uart::puts("[DEBUG] smoltcp requested TxToken\n");
        crate::drivers::uart::puts("[DEBUG] transmit() requested\n");
        Some(VirtioTxToken { queue: &mut self.tx, base: self.base })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1500;
        // Some smoltcp versions require an explicit burst size to trigger TX
        caps.max_burst_size = Some(1); 
        caps.medium = Medium::Ethernet;
        caps
    }
}

// --- 3. Token Implementations (UNCHANGED) ---

impl<'a> phy::RxToken for VirtioRxToken<'a> {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        unsafe {
            let last = self.queue.last_used_idx % self.queue.size;
            let used_elem = &(*self.queue.used).ring[last as usize];
            let desc_id = used_elem.id as u16;

            let desc_ptr = self.queue.desc.add(desc_id as usize);
            let data_ptr = ((*desc_ptr).addr + 12) as *mut u8;
            let data_len = (used_elem.len - 12) as usize;

            let slice = core::slice::from_raw_parts_mut(data_ptr, data_len);

            if data_len > 0 {
                crate::drivers::uart::puts("[DEBUG] RX packet consumed\n");
            }

            let result = f(slice);

            // --- CRITICAL: RE-QUEUE THE DESCRIPTOR ---
            let avail_slot = (*self.queue.avail).idx % self.queue.size;
            (*self.queue.avail).ring[avail_slot as usize] = desc_id;

            atomic::fence(Ordering::SeqCst);

            (*self.queue.avail).idx = (*self.queue.avail).idx.wrapping_add(1);

            atomic::fence(Ordering::SeqCst);

            self.queue.last_used_idx = self.queue.last_used_idx.wrapping_add(1);

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
            // 1. Get the current descriptor we want to use. 
            // For a simple driver, we can use descriptor 0, 
            // but we must ensure the length is updated.
            let desc_ptr = self.queue.desc.add(0);
            (*desc_ptr).len = (len + 12) as u32; // Include VirtIO Header

            // 2. Data starts after the 12-byte VirtIO header
            let data_ptr = ((*desc_ptr).addr + 12) as *mut u8;
            let slice = core::slice::from_raw_parts_mut(data_ptr, len);

            // 3. Let smoltcp fill the buffer with the Ethernet frame
            let result = f(slice);

            // 4. Update the Available Ring
            // We put descriptor 0 into the next available slot in the ring
            let avail_idx = (*self.queue.avail).idx % self.queue.size;
            (*self.queue.avail).ring[avail_idx as usize] = 0;

            // --- CRITICAL: MEMORY BARRIER ---
            // Ensure data is in RAM before updating the index
            atomic::fence(Ordering::SeqCst);

            // 5. Increment the Available Index to tell the device "1 new packet"
            (*self.queue.avail).idx = (*self.queue.avail).idx.wrapping_add(1);

            // --- CRITICAL: NOTIFY BARRIER ---
            // Ensure index is updated before ringing the doorbell
            atomic::fence(Ordering::SeqCst);

            // 6. Ring the Doorbell for Queue 1 (TX)
            core::ptr::write_volatile((self.base + 0x050) as *mut u32, 1);

            crate::drivers::uart::puts("[DEBUG] TX Packet pushed to VirtIO doorbell\n");
            
            result
        }
    }
}

// --- 4. WebUI Assets & Routing ---

pub const INDEX_HTML: &[u8] = include_bytes!("../../webui/index.html");
pub const STYLE_CSS:  &[u8] = include_bytes!("../../webui/style.css");
pub const APP_JS:     &[u8] = include_bytes!("../../webui/app.js");

/// Simple container for an HTTP response
pub struct Response {
    pub header: &'static [u8],
    pub body: &'static [u8],
}

/// Dispatches the correct file based on the HTTP request string
pub fn dispatch_request(request: &[u8]) -> Response {
    // Check for the path in the GET request (e.g., "GET /style.css HTTP/1.1")
    if contains(request, b"GET /style.css") {
        Response {
            header: b"HTTP/1.1 200 OK\r\nContent-Type: text/css\r\nConnection: close\r\n\r\n",
            body: STYLE_CSS,
        }
    } else if contains(request, b"GET /app.js") {
        Response {
            header: b"HTTP/1.1 200 OK\r\nContent-Type: application/javascript\r\nConnection: close\r\n\r\n",
            body: APP_JS,
        }
    } else {
        // Default to index.html for "/" or unknown paths
        Response {
            header: b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n",
            body: INDEX_HTML,
        }
    }
}

/// Minimal no_std helper to check if a byte slice contains a pattern
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.len() > haystack.len() { return false; }
    haystack.windows(needle.len()).any(|window| window == needle)
}

/// Helper to find the start of the path in a GET request
pub fn get_request_path(request: &[u8]) -> &[u8] {
    let mut start = 0;
    for i in 0..request.len() {
        if request[i..].starts_with(b"GET ") {
            start = i + 4;
            break;
        }
    }
    if start == 0 { return b"unknown"; }
    
    let mut end = start;
    while end < request.len() && request[end] != b' ' {
        end += 1;
    }
    &request[start..end]
}