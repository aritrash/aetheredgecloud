use core::ptr::{read_volatile, write_volatile};
use core::arch::asm;

// QEMU Virt GICv3 MMIO Bases
const GICD_BASE: usize = 0x08000000; // Distributor
const GICR_BASE: usize = 0x080A0000; // Redistributor (CPU 0)

// MMIO Offsets
const GICD_CTLR:   usize = 0x0000;
const GICR_WAKER:  usize = 0x0014;

pub fn init() {
    unsafe {
        // 1. Distributor: Enable Group 1 (Normal interrupts)
        // Bit 4 = ARE_NS (Enable Affinity Routing), Bit 1 = EnableGrp1NS
        write_volatile((GICD_BASE + GICD_CTLR) as *mut u32, (1 << 4) | (1 << 1));

        // 2. Redistributor: Wake up the CPU interface
        // We must clear the ProcessorSleep bit (Bit 1)
        let waker_addr = (GICR_BASE + GICR_WAKER) as *mut u32;
        let mut waker = read_volatile(waker_addr);
        waker &= !(1 << 1);
        write_volatile(waker_addr, waker);
        
        // Wait for ChildrenAsleep (Bit 2) to clear
        while (read_volatile(waker_addr) & (1 << 2)) != 0 {}

        // 3. CPU Interface: Enable System Register Access (ICC_SRE_EL1)
        // Set bit 0 (SRE) to 1.
        let mut sre: u64;
        asm!("mrs {}, ICC_SRE_EL1", out(reg) sre);
        asm!("msr ICC_SRE_EL1, {}", in(reg) sre | 1);
        asm!("isb");

        // 4. Set Priority Mask (ICC_PMR_EL1)
        // Allow all interrupts (0xFF)
        asm!("msr ICC_PMR_EL1, {}", in(reg) 0xFFu64);

        // 5. Enable Group 1 Interrupts (ICC_IGRPEN1_EL1)
        asm!("msr ICC_IGRPEN1_EL1, {}", in(reg) 1u64);
        asm!("isb");
    }
}

pub fn acknowledge_irq() -> u32 {
    let irq: u64;
    unsafe {
        // Read Interrupt Acknowledge Register for Group 1
        asm!("mrs {}, ICC_IAR1_EL1", out(reg) irq);
    }
    (irq & 0xFFFFFF) as u32
}

pub fn end_of_interrupt(irq: u32) {
    unsafe {
        // Write to End of Interrupt Register for Group 1
        asm!("msr ICC_EOIR1_EL1, {}", in(reg) irq as u64);
        asm!("isb");
    }
}