//! xHCI controller/ring model. Command, transfer and event rings are distinct.
use crate::device::{Device, Resource};

pub const XHCI_CLASS: u8 = 0x0c;
pub const XHCI_SUBCLASS: u8 = 0x03;
pub const XHCI_PROGIF: u8 = 0x30;
pub const XHCI_RING_TRBS: usize = 256;

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct Trb { pub parameter: u64, pub status: u32, pub control: u32 }
impl Trb { pub const EMPTY: Self = Self { parameter: 0, status: 0, control: 0 }; }

pub struct TrbRing {
    entries: [Trb; XHCI_RING_TRBS],
    enqueue: u16,
    dequeue: u16,
    cycle: bool,
}
impl TrbRing {
    pub const fn new() -> Self {
        Self { entries: [Trb::EMPTY; XHCI_RING_TRBS], enqueue: 0, dequeue: 0, cycle: true }
    }
    pub fn push(&mut self, mut t: Trb) -> Result<u16, &'static str> {
        let next = (self.enqueue + 1) % XHCI_RING_TRBS as u16;
        if next == self.dequeue { return Err("xHCI ring full"); }
        if self.cycle { t.control |= 1 } else { t.control &= !1 }
        let slot = self.enqueue;
        self.entries[slot as usize] = t;
        self.enqueue = next;
        if self.enqueue == 0 { self.cycle = !self.cycle; }
        Ok(slot)
    }
    pub fn complete_to(&mut self, index: u16) {
        self.dequeue = (index + 1) % XHCI_RING_TRBS as u16;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XhciState { Discovered, Halted, Resetting, Running, Failed }

pub struct XhciController {
    pub device: u64,
    pub mmio: u64,
    pub state: XhciState,
    pub max_slots: u8,
    pub scratchpads: u16,
    pub command: TrbRing,
    pub transfer: TrbRing,
    pub event: TrbRing,
}
impl XhciController {
    pub fn from_device(d: &Device) -> Result<Self, &'static str> {
        if d.class_code != XHCI_CLASS || d.subclass != XHCI_SUBCLASS || d.programming_interface != XHCI_PROGIF {
            return Err("not xHCI");
        }
        let mmio = d.resources.iter().find_map(|r| {
            if let Resource::Mmio { base, .. } = r { Some(*base) } else { None }
        }).ok_or("xHCI BAR missing")?;
        if mmio & 0xfff != 0 { return Err("xHCI BAR unaligned"); }
        Ok(Self {
            device: d.id.0,
            mmio,
            state: XhciState::Discovered,
            max_slots: 0,
            scratchpads: 0,
            command: TrbRing::new(),
            transfer: TrbRing::new(),
            event: TrbRing::new(),
        })
    }

    pub fn reset(&mut self) -> Result<(), &'static str> {
        self.state = XhciState::Resetting;
        self.command = TrbRing::new();
        self.transfer = TrbRing::new();
        self.event = TrbRing::new();
        self.state = XhciState::Halted;
        Ok(())
    }

    pub fn start(&mut self, max_slots: u8, scratchpads: u16) -> Result<(), &'static str> {
        if self.state != XhciState::Halted || max_slots == 0 { return Err("xHCI not ready"); }
        self.max_slots = max_slots;
        self.scratchpads = scratchpads;
        self.state = XhciState::Running;
        Ok(())
    }

    /// Build a USB control-transfer TD on a transfer ring. The xHCI command
    /// ring is reserved for host-controller commands and must never carry TDs.
    pub fn submit_control(&mut self, setup: [u8; 8], buffer: u64, length: u16) -> Result<(), &'static str> {
        if self.state != XhciState::Running { return Err("xHCI offline"); }
        if length != 0 && buffer == 0 { return Err("xHCI control transfer has null data buffer"); }
        let mut p = 0u64;
        for (i, b) in setup.iter().enumerate() { p |= (*b as u64) << (i * 8); }

        // Setup Stage TRB, immediate-data bit set.
        self.transfer.push(Trb { parameter: p, status: 8, control: (2 << 10) | (1 << 6) })?;
        if length > 0 {
            // Data Stage TRB. Direction is derived from bmRequestType bit 7.
            let direction_in = setup[0] & 0x80 != 0;
            self.transfer.push(Trb {
                parameter: buffer,
                status: length as u32,
                control: (3 << 10) | ((direction_in as u32) << 16),
            })?;
        }
        // Status Stage TRB with IOC. Status direction is opposite data direction.
        let status_in = length == 0 || setup[0] & 0x80 == 0;
        self.transfer.push(Trb {
            parameter: 0,
            status: 0,
            control: (4 << 10) | ((status_in as u32) << 16) | (1 << 5),
        })?;
        Ok(())
    }
}
