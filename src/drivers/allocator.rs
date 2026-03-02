use core::sync::atomic::{AtomicUsize, Ordering};

// 2MB for VirtQueues and Packet Buffers
const ARENA_SIZE: usize = 2 * 1024 * 1024;

// FIXED: Added repr(align) to ensure the base of the arena is page-aligned.
// Without this, 'aligned' offsets are calculated relative to an unknown base.
#[repr(align(4096))]
struct Arena {
    data: [u8; ARENA_SIZE],
}

static mut ARENA: Arena = Arena {
    data: [0; ARENA_SIZE],
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

pub fn allocate_aligned(size: usize, align: usize) -> *mut u8 {
    // 1. Load the current bump pointer
    let current = NEXT.load(Ordering::SeqCst);
    
    // 2. Calculate the next aligned address
    let aligned = (current + align - 1) & !(align - 1);
    
    // 3. Bound check
    if aligned + size > ARENA_SIZE {
        // In a kernel, we'd ideally trigger a UART warning here
        return core::ptr::null_mut();
    }
    
    // 4. Update the pointer for the next allocation
    NEXT.store(aligned + size, Ordering::SeqCst);
    
    // 5. Return the pointer from our aligned static arena
    unsafe {
        ARENA.data.as_mut_ptr().add(aligned)
    }
}

/// Helper to reset the allocator (useful if you ever need to re-init the NIC)
pub fn reset() {
    NEXT.store(0, Ordering::SeqCst);
}