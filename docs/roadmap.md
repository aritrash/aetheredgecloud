# Aether EdgeCloud Roadmap

This document outlines the phased development plan for Aether EdgeCloud.

The project is structured to build a stable EL2 micro-hypervisor first,
followed by guest lifecycle management, and finally distributed fabric orchestration.

---

# Phase 0 — Foundation (EL2 Hypervisor Skeleton)

Goal: Establish a stable EL2-resident hypervisor core.

Milestones:
- [ ] Remain in EL2 (remove EL2 → EL1 transition)
- [ ] Setup EL2 stack
- [ ] Implement EL2 exception vector table
- [ ] Decode and print ESR_EL2
- [ ] Confirm EL2 exception handling via SVC
- [ ] Clean hypervisor module structure

Deliverable:
Aether EdgeCloud boots and operates fully in EL2.

---

# Phase 1 — Stage-2 Memory Management

Goal: Implement minimal virtualization memory control.

Milestones:
- [ ] Define Stage-2 translation tables
- [ ] Identity-map physical memory via Stage-2
- [ ] Configure VTCR_EL2
- [ ] Set VTTBR_EL2
- [ ] Enable HCR_EL2.VM
- [ ] Verify stable execution after enabling virtualization

Deliverable:
Hypervisor manages Stage-2 address translation successfully.

---

# Phase 2 — Guest Boot (Minimal EL1 Stub)

Goal: Launch a controlled EL1 guest.

Milestones:
- [ ] Allocate guest memory region
- [ ] Setup EL1 stack pointer
- [ ] Configure SPSR_EL2 for EL1h
- [ ] Set ELR_EL2 to guest entry point
- [ ] ERET into EL1
- [ ] Guest prints to UART

Deliverable:
Hypervisor successfully launches an EL1 guest stub.

---

# Phase 3 — Linux Guest Boot

Goal: Boot a real Linux kernel as guest.

Milestones:
- [ ] Load Linux Image into guest memory
- [ ] Provide DTB or boot parameters
- [ ] Minimal virtual console
- [ ] Verify Linux boots to CLI
- [ ] Confirm isolation boundaries

Deliverable:
Linux runs as an EL1 guest under Aether EdgeCloud.

---

# Phase 4 — Hypervisor Control Plane

Goal: Enable VM lifecycle control and telemetry.

Milestones:
- [ ] VM start/stop API
- [ ] Basic metrics (CPU, memory usage)
- [ ] Virtual interrupt routing
- [ ] Timer virtualization
- [ ] Resource quota enforcement

Deliverable:
Hypervisor supports controlled VM lifecycle operations.

---

# Phase 5 — Fabric Controller (Single Master Node)

Goal: Implement centralized cluster orchestration.

Milestones:
- [ ] Node discovery protocol
- [ ] Secure node registration
- [ ] Job scheduling engine
- [ ] Workload dispatching
- [ ] Storage partition management
- [ ] Web dashboard API (HTTP control plane)

Deliverable:
Single-controller cluster fabric operational.

---

# Phase 6 — Distributed Fabric (Future)

Goal: Remove single point of failure.

Milestones:
- [ ] Leader election (Raft or similar)
- [ ] Distributed state replication
- [ ] Failure detection
- [ ] Workload migration
- [ ] Multi-controller redundancy

Deliverable:
Resilient distributed edge cloud fabric.

---

# Long-Term Goals

- Secure boot chain
- Hardware attestation
- Multi-tenant isolation
- Live migration
- Lightweight VM boot (<100ms)
- Secure remote provisioning
- ARM accelerator passthrough

---

# Development Philosophy

Build hypervisor first.
Prove isolation.
Then add orchestration.
Never compromise EL2 minimalism.