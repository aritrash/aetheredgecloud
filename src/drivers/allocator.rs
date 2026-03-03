use core::sync::atomic::{AtomicUsize, Ordering};
use core::ptr::{null_mut, write_bytes};

/// 8MB Arena for VirtQueues, Framebuffers, and Packet Buffers.
/// Increased from 2MB to 8MB to safely house a 1024x768 BGRA framebuffer (~3MB).
const ARENA_SIZE: usize = 8 * 1024 * 1024;

#[repr(align(4096))]
struct Arena {
    data: [u8; ARENA_SIZE],
}

/// The static storage for all dynamic kernel allocations.
static mut ARENA: Arena = Arena {
    data: [0; ARENA_SIZE],
};

/// Atomic bump pointer tracking the current offset into the Arena.
static NEXT: AtomicUsize = AtomicUsize::new(0);

/// Allocates a block of memory with a specific alignment.
/// Automatically zero-initializes the memory to prevent "ghost" data issues.
pub fn allocate_aligned(size: usize, align: usize) -> *mut u8 {
    // We use fetch_update to make the "Calculate + Store" operation atomic.
    // This prevents race conditions if you ever enable multi-core (SMP).
    let allocation_result = NEXT.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
        // 1. Calculate the next aligned offset
        let aligned = (current + align - 1) & !(align - 1);
        
        // 2. Bound check: If we exceed the arena, we fail the update
        if aligned + size > ARENA_SIZE {
            None
        } else {
            // 3. Update the pointer to the end of this new block
            Some(aligned + size)
        }
    });

    match allocation_result {
        Ok(prev_end_offset) => {
            // fetch_update returns the OLD value. We need to re-calculate 
            // the aligned start based on that old value.
            let start_offset = (prev_end_offset + align - 1) & !(align - 1);
            
            unsafe {
                let ptr = ARENA.data.as_mut_ptr().add(start_offset);
                
                // CRITICAL: Zero-initialize the memory. 
                // VirtIO queues MUST be zeroed before enabling or the device 
                // might read old 'idx' values from a previous QEMU run.
                write_bytes(ptr, 0, size);
                
                ptr
            }
        }
        Err(_) => {
            // In a bare-metal kernel, this is usually a "Panic" condition.
            null_mut()
        }
    }
}

/// Resets the allocator. 
/// USE WITH CAUTION: This invalidates all existing pointers.
pub fn reset() {
    NEXT.store(0, Ordering::SeqCst);
}