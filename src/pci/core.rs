use crate::drivers::uart;
use crate::pci::host::PciHost;

const PCI_STATUS_CAP_LIST: u16 = 1 << 4;
const PCI_CAP_ID_VENDOR: u8 = 0x09;

const PCI_BAR4_OFFSET: u16 = 0x20;
const PCI_BAR5_OFFSET: u16 = 0x24;
const PCI_COMMAND_OFFSET: u16 = 0x04;

static mut NEXT_MMIO_BASE: u64 = 0x1000_0000;

fn align_up(val: u64, align: u64) -> u64 {
    (val + align - 1) & !(align - 1)
}

pub struct VirtioPciDeviceInfo {
    pub common_cfg: usize,
    pub notify_base: usize,
    pub notify_off_multiplier: u32,
}

pub unsafe fn enumerate(host: &dyn PciHost)
    -> Option<VirtioPciDeviceInfo>
{
    uart::puts("[PCI] Enumerating bus 0...\n");

    for dev in 0u8..32 {

        let id = host.read(0, dev, 0, 0x00);
        let vendor = (id & 0xFFFF) as u16;

        if vendor == 0xFFFF || vendor == 0 {
            continue;
        }

        let device = ((id >> 16) & 0xFFFF) as u16;

        uart::puts("[PCI] Found device: ");
        uart::putc_hex64(vendor as u64);
        uart::puts(":");
        uart::putc_hex64(device as u64);
        uart::puts("\n");

        if vendor != 0x1AF4 || device != 0x1050 {
            continue;
        }

        uart::puts("[PCI] -> VirtIO GPU detected\n");

        // ---- BAR sizing ----

        let orig_lo = host.read(0, dev, 0, PCI_BAR4_OFFSET);
        let orig_hi = host.read(0, dev, 0, PCI_BAR5_OFFSET);

        host.write(0, dev, 0, PCI_BAR4_OFFSET, 0xFFFF_FFFF);
        host.write(0, dev, 0, PCI_BAR5_OFFSET, 0xFFFF_FFFF);

        let size_lo = host.read(0, dev, 0, PCI_BAR4_OFFSET);
        let size_hi = host.read(0, dev, 0, PCI_BAR5_OFFSET);

        host.write(0, dev, 0, PCI_BAR4_OFFSET, orig_lo);
        host.write(0, dev, 0, PCI_BAR5_OFFSET, orig_hi);

        let mask =
            ((size_hi as u64) << 32) |
            ((size_lo as u64) & 0xFFFF_FFF0);

        let bar_size = (!mask).wrapping_add(1);

        let assigned =
            align_up(NEXT_MMIO_BASE, bar_size);

        NEXT_MMIO_BASE = assigned + bar_size;

        host.write(
            0, dev, 0,
            PCI_BAR4_OFFSET,
            (assigned as u32) | 0x4,
        );

        host.write(
            0, dev, 0,
            PCI_BAR5_OFFSET,
            (assigned >> 32) as u32,
        );

        let cmd = host.read(0, dev, 0, PCI_COMMAND_OFFSET);
        host.write(0, dev, 0, PCI_COMMAND_OFFSET, cmd | 0x2);

        let bar_lo = host.read(0, dev, 0, PCI_BAR4_OFFSET);
        let bar_hi = host.read(0, dev, 0, PCI_BAR5_OFFSET);

        let bar_base =
            ((bar_hi as u64) << 32) |
            ((bar_lo as u64) & 0xFFFF_FFF0);

        // ---- capability walk ----

        let status =
            ((host.read(0, dev, 0, 0x04) >> 16) & 0xFFFF) as u16;

        if (status & PCI_STATUS_CAP_LIST) == 0 {
            continue;
        }

        let mut cap_ptr =
            (host.read(0, dev, 0, 0x34) & 0xFF) as u8;

        let mut common_cfg = None;
        let mut notify_base = None;
        let mut notify_multiplier = 0;

        while cap_ptr != 0 {

            let header =
                host.read(0, dev, 0, cap_ptr as u16);

            let cap_id = (header & 0xFF) as u8;
            let next = ((header >> 8) & 0xFF) as u8;

            if cap_id == PCI_CAP_ID_VENDOR {

                let cap = host.read(0, dev, 0, cap_ptr as u16);
                let cfg_type =
                    ((cap >> 24) & 0xFF) as u8;

                let offset =
                    host.read(0, dev, 0, cap_ptr as u16 + 0x08);

                if cfg_type == 1 {
                    uart::puts("[PCI] Found common config\n");
                    common_cfg =
                        Some(bar_base + offset as u64);
                }

                if cfg_type == 2 {
                    uart::puts("[PCI] Found notify config\n");

                    notify_base =
                        Some(bar_base + offset as u64);

                    notify_multiplier =
                        host.read(0, dev, 0,
                            cap_ptr as u16 + 0x10);
                }
            }

            cap_ptr = next;
        }

        if let (Some(c), Some(n)) =
            (common_cfg, notify_base)
        {
            uart::puts("[PCI] Returning VirtIO PCI transport info\n");

            return Some(VirtioPciDeviceInfo {
                common_cfg: c as usize,
                notify_base: n as usize,
                notify_off_multiplier: notify_multiplier,
            });
        }
    }

    None
}