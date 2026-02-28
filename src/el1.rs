use crate::drivers::{uart, virtio_net};

#[no_mangle]
pub extern "C" fn el1_rust_main() {
    uart::puts("\n[EL1] Rust main started\n");

    virtio_net::probe();

    loop {}
}