use crate::drivers::uart;
use crate::pci::host::PciHost;
use core::ptr::{read_volatile, write_volatile};

use crate::drivers::virtio_pci::VirtioPciCommonCfg;

const PCI_STATUS_CAP_LIST: u16 = 1 << 4;
const PCI_CAP_ID_VENDOR: u8 = 0x09;

const PCI_COMMAND_OFFSET: u16 = 0x04;
const PCI_BAR4_OFFSET: u16 = 0x20;
const PCI_BAR5_OFFSET: u16 = 0x24;

static mut NEXT_PCI_MMIO: u64 = 0x1000_0000;

fn align_up(value: u64, align: u64) -> u64 {
    (value + align - 1) & !(align - 1)
}

pub unsafe fn enumerate(host: &dyn PciHost) {
    uart::puts("[PCI] Enumerating bus 0...\n");

    for dev in 0u8..32 {
        let id = host.read(0, dev, 0, 0x00);
        let vendor = (id & 0xFFFF) as u16;

        if vendor == 0xFFFF || vendor == 0x0000 {
            continue;
        }

        let device = ((id >> 16) & 0xFFFF) as u16;

        uart::puts("[PCI] Found device: ");
        uart::putc_hex64(vendor as u64);
        uart::puts(":");
        uart::putc_hex64(device as u64);
        uart::puts("\n");

        if vendor == 0x1AF4 && device == 0x1050 {
            uart::puts("[PCI] -> VirtIO GPU detected\n");

            // ---------------- BAR PROBE ----------------

            let original_bar4 = host.read(0, dev, 0, PCI_BAR4_OFFSET);
            let original_bar5 = host.read(0, dev, 0, PCI_BAR5_OFFSET);

            host.write(0, dev, 0, PCI_BAR4_OFFSET, 0xFFFF_FFFF);
            host.write(0, dev, 0, PCI_BAR5_OFFSET, 0xFFFF_FFFF);

            let size_low = host.read(0, dev, 0, PCI_BAR4_OFFSET);
            let size_high = host.read(0, dev, 0, PCI_BAR5_OFFSET);

            host.write(0, dev, 0, PCI_BAR4_OFFSET, original_bar4);
            host.write(0, dev, 0, PCI_BAR5_OFFSET, original_bar5);

            let size_mask =
                ((size_high as u64) << 32) |
                ((size_low as u64) & 0xFFFF_FFF0);

            let bar_size = (!size_mask).wrapping_add(1);

            uart::puts("[PCI] BAR4 size detected: ");
            uart::putc_hex64(bar_size);
            uart::puts("\n");

            let assigned_base = align_up(NEXT_PCI_MMIO, bar_size);
            NEXT_PCI_MMIO = assigned_base + bar_size;

            uart::puts("[PCI] Assigned BAR4 base: ");
            uart::putc_hex64(assigned_base);
            uart::puts("\n");

            host.write(
                0,
                dev,
                0,
                PCI_BAR4_OFFSET,
                (assigned_base as u32) | 0x4,
            );

            host.write(
                0,
                dev,
                0,
                PCI_BAR5_OFFSET,
                (assigned_base >> 32) as u32,
            );

            let command = host.read(0, dev, 0, PCI_COMMAND_OFFSET);
            host.write(0, dev, 0, PCI_COMMAND_OFFSET, command | 0x2);

            let bar4_low = host.read(0, dev, 0, PCI_BAR4_OFFSET);
            let bar5_high = host.read(0, dev, 0, PCI_BAR5_OFFSET);

            let bar_base =
                ((bar5_high as u64) << 32) |
                ((bar4_low as u64) & 0xFFFF_FFF0);

            uart::puts("[PCI] BAR4 final base: ");
            uart::putc_hex64(bar_base);
            uart::puts("\n");

            // ---------------- CAPABILITY WALK ----------------

            let status_reg = host.read(0, dev, 0, 0x04);
            let status = ((status_reg >> 16) & 0xFFFF) as u16;

            if (status & PCI_STATUS_CAP_LIST) == 0 {
                uart::puts("[PCI] ERROR: No capability list!\n");
                continue;
            }

            let cap_ptr_raw = host.read(0, dev, 0, 0x34);
            let mut cap_ptr = (cap_ptr_raw & 0xFF) as u8;

            let mut common_cfg_addr: Option<u64> = None;

            while cap_ptr != 0 {
                let cap_header = host.read(0, dev, 0, cap_ptr as u16);

                let cap_id = (cap_header & 0xFF) as u8;
                let next_ptr = ((cap_header >> 8) & 0xFF) as u8;

                if cap_id == PCI_CAP_ID_VENDOR {
                    let cap_reg =
                        host.read(0, dev, 0, cap_ptr as u16);

                    let cfg_type =
                        ((cap_reg >> 24) & 0xFF) as u8;

                    let offset =
                        host.read(0, dev, 0, cap_ptr as u16 + 0x08);

                    if cfg_type == 1 {
                        common_cfg_addr =
                            Some(bar_base + offset as u64);
                    }
                }

                cap_ptr = next_ptr;
            }

            // ---------------- COMMON CONFIG INIT ----------------

            if let Some(addr) = common_cfg_addr {
                let common_cfg = addr as *mut VirtioPciCommonCfg;

                // --------------------------------------------------
                // 1. Reset device
                // --------------------------------------------------
                write_volatile(&mut (*common_cfg).device_status, 0);

                // --------------------------------------------------
                // 2. ACKNOWLEDGE
                // --------------------------------------------------
                write_volatile(&mut (*common_cfg).device_status, 1);

                // --------------------------------------------------
                // 3. DRIVER
                // --------------------------------------------------
                write_volatile(&mut (*common_cfg).device_status, 1 | 2);

                // --------------------------------------------------
                // 4. Read device features
                // --------------------------------------------------
                write_volatile(&mut (*common_cfg).device_feature_select, 0);
                let device_features =
                    read_volatile(&(*common_cfg).device_feature);

                uart::puts("[VIRTIO] Device features: ");
                uart::putc_hex64(device_features as u64);
                uart::puts("\n");

                // --------------------------------------------------
                // 5. Negotiate features
                //    Accept ONLY VIRTIO_F_VERSION_1 (bit 0)
                // --------------------------------------------------
                write_volatile(&mut (*common_cfg).driver_feature_select, 0);
                write_volatile(&mut (*common_cfg).driver_feature, 1);

                // --------------------------------------------------
                // 6. Set FEATURES_OK
                // --------------------------------------------------
                let mut status =
                    read_volatile(&(*common_cfg).device_status);

                status |= 8; // FEATURES_OK
                write_volatile(&mut (*common_cfg).device_status, status);

                // Verify FEATURES_OK
                let status_check =
                    read_volatile(&(*common_cfg).device_status);

                uart::puts("[VIRTIO] Status after FEATURES_OK: ");
                uart::putc_hex64(status_check as u64);
                uart::puts("\n");

                // If device cleared FEATURES_OK, negotiation failed
                if (status_check & 8) == 0 {
                    uart::puts("[VIRTIO] ERROR: FEATURES_OK rejected\n");
                    return;
                }

                // --------------------------------------------------
                // 7. DRIVER_OK
                // --------------------------------------------------
                status |= 4; // DRIVER_OK
                write_volatile(&mut (*common_cfg).device_status, status);

                let final_status =
                    read_volatile(&(*common_cfg).device_status);

                uart::puts("[VIRTIO] Final device status: ");
                uart::putc_hex64(final_status as u64);
                uart::puts("\n");
            }
        }
    }
}