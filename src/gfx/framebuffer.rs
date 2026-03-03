use core::ptr::write_volatile;

pub struct Framebuffer {
    pub base: *mut u8,
    pub width: usize,
    pub height: usize,
    pub stride: usize,
}

impl Framebuffer {
    pub fn new(base: usize, width: usize, height: usize, stride: usize) -> Self {
        Self {
            base: base as *mut u8,
            width,
            height,
            stride,
        }
    }

    pub fn clear(&self, color: u32) {
        for y in 0..self.height {
            for x in 0..self.width {
                let offset = y * self.stride + x * 4;
                unsafe {
                    write_volatile(self.base.add(offset) as *mut u32, color);
                }
            }
        }
    }
}