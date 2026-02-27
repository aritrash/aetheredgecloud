# Aether EdgeCloud Boot Flow

This document describes the intended boot sequence for Aether EdgeCloud.

The hypervisor operates at EL2 and retains control over hardware resources.
Guests execute at EL1.

---

# 1. System Reset

CPU starts in EL2 (expected on QEMU virt / ARMv8 platforms).

Initial state:
- MMU disabled
- Caches disabled
- Interrupts masked

---

# 2. Early EL2 Initialization

- Detect CurrentEL
- Confirm EL2 execution
- Configure EL2 stack pointer
- Enable FP/SIMD access (CPTR_EL2)
- Configure CNTHCTL_EL2
- Clear BSS
- Setup minimal runtime environment

---

# 3. EL2 Exception Vectors

- Define EL2 vector table
- Align table to 2048 bytes
- Set VBAR_EL2
- ISB barrier

At this stage:
Hypervisor can handle EL2 synchronous exceptions, IRQ, FIQ, SError.

---

# 4. Stage-2 Translation Setup

- Allocate Stage-2 page tables
- Define memory attributes
- Configure VTCR_EL2
- Set VTTBR_EL2
- Enable virtualization in HCR_EL2
- ISB barrier

Stage-2 identity mapping initially:
Guest physical = Host physical (controlled)

---

# 5. Hypervisor Ready State

Hypervisor now:
- Runs in EL2
- Controls Stage-2 memory
- Handles exceptions
- Can allocate guest memory
- Can prepare guest context

---

# 6. Guest Launch Sequence

To launch a guest:

- Allocate guest memory region
- Load guest image into memory
- Set SP_EL1 (guest stack)
- Set ELR_EL2 to guest entry
- Configure SPSR_EL2 for EL1h
- Configure HCR_EL2 for EL1 execution
- ERET

CPU transitions:
EL2 → EL1 (guest context)

---

# 7. Guest Execution

Guest runs at EL1:

- Uses Stage-1 translation (Linux-managed)
- Hardware access controlled by Stage-2
- Traps routed back to EL2 when configured

---

# 8. Trap Handling

When guest traps:

- CPU switches to EL2
- ESR_EL2 captures exception class
- Hypervisor decides:
  - Emulate
  - Forward
  - Inject virtual interrupt
  - Terminate guest

---

# 9. Control Plane Interaction

On central node:

- Hypervisor exposes management interface
- Control plane runs in management VM (EL1)
- Web dashboard served via HTTP
- Fabric commands sent to worker nodes

---

# Boot Flow Philosophy

Hypervisor must:

- Remain minimal
- Avoid feature bloat
- Prioritize isolation correctness
- Separate control plane from hypervisor core

EL2 is infrastructure.
EL1 is execution.