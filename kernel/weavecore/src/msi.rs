//! PCI MSI/MSI-X capability discovery and programming primitives.
use crate::{pci, pci_address::PciAddress};
use core::ptr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterruptCapability {
    None,
    Msi { offset: u8, multi_message: u8, is_64: bool, maskable: bool },
    MsiX { offset: u8, table_size: u16, table_bar: u8, table_offset: u32, pba_bar: u8, pba_offset: u32 },
}

pub fn discover(a: PciAddress) -> Result<InterruptCapability, &'static str> {
    if a.segment != 0 { return Err("legacy PCI config supports segment zero only"); }
    let status = (pci::read_u32(a.bus, a.device, a.function, 0x04) >> 16) as u16;
    if status & (1 << 4) == 0 { return Ok(InterruptCapability::None); }
    let mut ptr_off = (pci::read_u32(a.bus, a.device, a.function, 0x34) & 0xfc) as u8;
    let mut seen = 0;
    while ptr_off >= 0x40 && seen < 48 {
        let header = pci::read_u32(a.bus, a.device, a.function, ptr_off);
        let id = header as u8;
        let next = ((header >> 8) & 0xfc) as u8;
        match id {
            0x05 => {
                let control = (header >> 16) as u16;
                return Ok(InterruptCapability::Msi {
                    offset: ptr_off,
                    multi_message: ((control >> 1) & 7) as u8,
                    is_64: control & (1 << 7) != 0,
                    maskable: control & (1 << 8) != 0,
                });
            }
            0x11 => {
                let control = (header >> 16) as u16;
                let table = pci::read_u32(a.bus, a.device, a.function, ptr_off + 4);
                let pba = pci::read_u32(a.bus, a.device, a.function, ptr_off + 8);
                return Ok(InterruptCapability::MsiX {
                    offset: ptr_off,
                    table_size: (control & 0x7ff) + 1,
                    table_bar: (table & 7) as u8,
                    table_offset: table & !7,
                    pba_bar: (pba & 7) as u8,
                    pba_offset: pba & !7,
                });
            }
            _ => {}
        }
        if next == 0 || next == ptr_off { break; }
        ptr_off = next;
        seen += 1;
    }
    Ok(InterruptCapability::None)
}

pub fn message(cpu_apic_id: u32, vector: u8) -> Result<(u64, u32), &'static str> {
    if vector < 0x20 { return Err("invalid MSI vector"); }
    Ok((0xfee0_0000u64 | ((cpu_apic_id as u64) << 12), vector as u32))
}

pub struct MsiLease {
    pub address: PciAddress,
    pub vector: u8,
    pub capability_offset: u8,
    pub enabled: bool,
    pub msix: bool,
}
impl MsiLease {
    pub fn disable(&mut self) {
        if !self.enabled || self.address.segment != 0 { return; }
        let header = pci::read_u32(self.address.bus, self.address.device, self.address.function, self.capability_offset);
        let mut control = (header >> 16) as u16;
        if self.msix { control &= !(1 << 15); } else { control &= !1; }
        pci::write_u32(
            self.address.bus, self.address.device, self.address.function, self.capability_offset,
            (header & 0xffff) | ((control as u32) << 16),
        );
        self.enabled = false;
    }
}

/// Program a single-message MSI vector into PCI configuration space.
pub fn enable_msi(a: PciAddress, cpu_apic_id: u32, vector: u8) -> Result<MsiLease, &'static str> {
    let InterruptCapability::Msi { offset, is_64, .. } = discover(a)? else {
        return Err("device has no MSI capability");
    };
    let (address, data) = message(cpu_apic_id, vector)?;
    pci::write_u32(a.bus, a.device, a.function, offset + 4, address as u32);
    let data_offset = if is_64 {
        pci::write_u32(a.bus, a.device, a.function, offset + 8, (address >> 32) as u32);
        offset + 12
    } else {
        offset + 8
    };
    let old_data = pci::read_u32(a.bus, a.device, a.function, data_offset);
    pci::write_u32(a.bus, a.device, a.function, data_offset, (old_data & 0xffff_0000) | (data & 0xffff));

    let header = pci::read_u32(a.bus, a.device, a.function, offset);
    let mut control = (header >> 16) as u16;
    control &= !(0x7 << 4); // MME = one vector even if MMC advertises more.
    control |= 1;          // MSI Enable.
    pci::write_u32(a.bus, a.device, a.function, offset, (header & 0xffff) | ((control as u32) << 16));
    Ok(MsiLease { address: a, vector, capability_offset: offset, enabled: true, msix: false })
}

/// Program one MSI-X table entry. `table_base` is the already mapped physical
/// BAR base plus the capability's table offset.
pub unsafe fn enable_msix_entry(
    a: PciAddress,
    table_base: u64,
    table_index: u16,
    cpu_apic_id: u32,
    vector: u8,
) -> Result<MsiLease, &'static str> {
    let InterruptCapability::MsiX { offset, table_size, .. } = discover(a)? else {
        return Err("device has no MSI-X capability");
    };
    if table_index >= table_size { return Err("MSI-X table index out of range"); }
    if table_base == 0 || table_base & 0xf != 0 { return Err("MSI-X table base is invalid"); }
    let (address, data) = message(cpu_apic_id, vector)?;
    let entry = (table_base + table_index as u64 * 16) as *mut u32;

    // Mask the entry during reprogramming, then publish address/data and unmask.
    unsafe {
        ptr::write_volatile(entry.add(3), 1);
        ptr::write_volatile(entry.add(0), address as u32);
        ptr::write_volatile(entry.add(1), (address >> 32) as u32);
        ptr::write_volatile(entry.add(2), data);
    }
    let header = pci::read_u32(a.bus, a.device, a.function, offset);
    let mut control = (header >> 16) as u16;
    control |= 1 << 15;     // MSI-X Enable.
    control &= !(1 << 14);  // Clear Function Mask.
    pci::write_u32(a.bus, a.device, a.function, offset, (header & 0xffff) | ((control as u32) << 16));
    unsafe { ptr::write_volatile(entry.add(3), 0); }

    Ok(MsiLease { address: a, vector, capability_offset: offset, enabled: true, msix: true })
}
