pub mod boot;
pub mod vectors;
use core::arch::asm;

/// Returns the current system time in Milliseconds since boot.
pub fn get_current_time_ms() -> u64 {
    let cntpct: u64;
    let cntfrq: u64;
    unsafe {
        // 1. Read the current physical counter value
        asm!("mrs {}, cntpct_el0", out(reg) cntpct);
        // 2. Read the frequency (ticks per second)
        asm!("mrs {}, cntfrq_el0", out(reg) cntfrq);
    }
    
    // Formula: (Ticks * 1000) / Frequency = Milliseconds
    // We use u128 for the intermediate product to prevent overflow 
    // if the kernel stays up for a long time.
    ((cntpct as u128 * 1000) / cntfrq as u128) as u64
}