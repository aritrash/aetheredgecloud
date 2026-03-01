use core::sync::atomic::{AtomicUsize, Ordering};

// 2MB for VirtQueues and Packet Buffers
const ARENA_SIZE: usize = 2 * 1024 * 1024;
static mut ARENA: [u8; ARENA_SIZE] = [0; ARENA_SIZE];
static NEXT: AtomicUsize = AtomicUsize::new(0);

pub fn allocate_aligned(size: usize, align: usize) -> *mut u8 {
    let current = NEXT.load(Ordering::Relaxed);
    let aligned = (current + align - 1) & !(align - 1);
    
    if aligned + size > ARENA_SIZE {
        return core::ptr::null_mut();
    }
    
    NEXT.store(aligned + size, Ordering::Relaxed);
    unsafe { ARENA.as_mut_ptr().add(aligned) }
}