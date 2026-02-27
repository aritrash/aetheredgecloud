# Aether EdgeCloud

Aether EdgeCloud is an ARM64 EL2 micro-hypervisor designed to power distributed edge compute fabrics.

It provides hardware-level isolation, workload orchestration primitives, and cluster coordination, while delegating execution to Linux guest virtual machines.

This project targets ARM-based compute nodes (e.g., Raspberry Pi clusters, ARM servers) and aims to build a secure, minimal, and scalable edge cloud infrastructure platform.

---

## Vision

Aether EdgeCloud is not a Linux replacement.

It is an infrastructure layer.

It sits at EL2 (Hypervisor level) on ARMv8-A (AArch64) and provides:

- Hardware-enforced isolation
- Stage-2 memory management
- Guest lifecycle control
- Resource partitioning
- Deterministic workload boundaries
- Distributed fabric orchestration (future)

Linux runs as a guest at EL1 and is used purely as a workload execution substrate.

The control plane (web dashboard, scheduling, fabric logic) is hosted on a central node and served via HTTP to remote clients.

---

## Architecture Overview

Each node in the fabric runs:

EL2 → Aether EdgeCloud Hypervisor  
EL1 → Linux Guest (CLI-only, no GUI)  

Central Controller Node:
- Hosts the web dashboard (HTTP API)
- Performs job scheduling
- Manages storage partitioning
- Monitors node health
- Coordinates workload distribution

Worker Nodes:
- Run Linux guest VMs
- Execute assigned workloads (AI/ML, data processing, compute jobs)
- Report telemetry back to controller

---

## Design Principles

- Minimal Trusted Computing Base (TCB)
- Hypervisor-first design (EL2 resident)
- Clear separation between control plane and execution plane
- Deterministic and observable infrastructure
- No GUI stack inside guests
- Headless operation by default
- Hardware-level isolation over namespace-based isolation
- Modular and layered architecture

---

## Why Not Just Ubuntu?

Ubuntu Server provides a general-purpose OS.

Aether EdgeCloud provides:

- Hardware-level isolation via EL2
- Lightweight hypervisor control
- Deterministic resource partitioning
- Fabric-wide orchestration primitives
- Minimal attack surface hypervisor layer
- Edge-cluster-first architecture

Linux remains the workload engine.
Aether EdgeCloud becomes the infrastructure fabric.

---

## Current Status

Early-stage hypervisor development.

Initial Milestones:

- [ ] EL2-first boot (remain in EL2)
- [ ] EL2 exception vector table
- [ ] Stage-2 identity mapping
- [ ] Launch minimal EL1 guest stub
- [ ] Boot Linux as guest
- [ ] Basic node telemetry
- [ ] Fabric controller prototype

---

## Target Platforms

- ARMv8-A (AArch64)
- Raspberry Pi (future hardware testing)
- QEMU virt (development target)

---

## Development Philosophy

Aether EdgeCloud is built incrementally:

1. Hypervisor core stability
2. Stage-2 memory control
3. Guest lifecycle management
4. Minimal virtual device support
5. Fabric coordination layer
6. Distributed scheduling and storage orchestration

We build infrastructure first.
Features later.

---

## Long-Term Goals

- Secure multi-tenant ARM edge clusters
- Lightweight compute orchestration for AI/ML workloads
- Minimal hypervisor with strong isolation guarantees
- Cluster-level resource scheduling
- Remote management via browser-based control plane
- Secure boot and hardware attestation (future)

---

## License

Apache License

---

## Author

Aether EdgeCloud is part of the broader Aether systems initiative.

Built for serious systems engineering, distributed infrastructure research, and ARM-based compute fabrics.