# Contributing to Aether EdgeCloud 

Thank you for contributing to Aether EdgeCloud! We are building a high-performance, Rust-based Server OS for Aarch64. To maintain a stable kernel and a clean codebase, please follow these guidelines.

## 1. Development Environment
Before contributing, ensure your environment is set up for cross-compilation:

- **Rust Toolchain:** `rustup target add aarch64-unknown-none`
- **Binary Utilities:** `cargo install cargo-binutils`
- **Emulator:** `qemu-system-aarch64` (Version 7.0+)
- **Build Tool:** `make`

## 2. Branching & Workflow
We follow a **Feature Branch** workflow. Direct pushes to `main` are blocked.

1. **Pick an Issue:** Find an open issue in the "Issues" tab. If you're starting something new, create an issue first to discuss the architecture.
2. **Create a Branch:** Name your branch based on the component:
   - `arch/` — e.g., `arch/mmu-init`
   - `feat/` — e.g., `feat/hypervisor-interface`
   - `driver/` — e.g., `driver/pl011-uart`
   - `fix/` — e.g., `fix/vector-table-alignment`
3. **Commit often:** Use descriptive commit messages (e.g., `feat(arch): implement EL2 to EL1 transition`).

## 3. Rust Coding Standards
We leverage Rust's type system to ensure kernel safety.
- **No Panic:** Avoid `panic!`, `unwrap()`, and `expect()` in core kernel code. Return `Result` or `Option` instead.
- **Safety Documentation:** Every `unsafe` block **MUST** be preceded by a `// SAFETY:` comment explaining why the operation is sound.
- **Formatting:** Code must be formatted using `cargo fmt`.
- **Lints:** Check your code with `cargo clippy --target aarch64-unknown-none`.

## 4. Pull Request (PR) Process
1. **Verify the Build:** Run `make clean && make run`. Ensure the kernel boots to the expected state in QEMU.
2. **Open a PR:** Use the provided template. Link it to the relevant issue using "Fixes #XX".
3. **Status Checks:** Our CI will automatically check formatting and compilation. These must pass before review.
4. **Review:** At least one approval from the Project Lead is required for merging.

## 5. Testing on QEMU
Always test your changes using the provided Makefile:
- `make clean` - Cleans the build and previous image binary.
- `cargo build` - Builds and generates the `edgecloud.img` binary.
- `make run` - Launches QEMU with the `virt` machine and GICv3.

---
*Aether OS Project - "Built for the Edge, Powered by Rust"*