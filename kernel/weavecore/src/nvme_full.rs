//! NVMe queue model with CID-accurate completion tracking and safe command validation.
use crate::{
    block_queue::BlockOperation,
    device::{Device, Resource},
};

pub const NVME_QUEUE_ENTRIES: usize = 128;
pub const MAX_NVME_NAMESPACES: usize = 32;
const PAGE_SIZE: u64 = 4096;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Submission {
    pub opcode: u8,
    pub flags: u8,
    pub cid: u16,
    pub nsid: u32,
    pub reserved: u64,
    pub metadata: u64,
    pub prp1: u64,
    pub prp2: u64,
    pub cdw10: u32,
    pub cdw11: u32,
    pub cdw12: u32,
    pub cdw13: u32,
    pub cdw14: u32,
    pub cdw15: u32,
}
impl Submission {
    pub const EMPTY: Self = Self {
        opcode: 0, flags: 0, cid: 0, nsid: 0, reserved: 0, metadata: 0,
        prp1: 0, prp2: 0, cdw10: 0, cdw11: 0, cdw12: 0, cdw13: 0,
        cdw14: 0, cdw15: 0,
    };
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Completion {
    pub result: u32,
    pub reserved: u32,
    pub sq_head: u16,
    pub sq_id: u16,
    pub cid: u16,
    pub status: u16,
}
impl Completion {
    pub const EMPTY: Self = Self {
        result: 0, reserved: 0, sq_head: 0, sq_id: 0, cid: 0, status: 0,
    };
    pub fn phase(self) -> bool { self.status & 1 != 0 }
    pub fn success(self) -> bool { (self.status >> 1) & 0x7ff == 0 }
}

pub struct Queue {
    sq: [Submission; NVME_QUEUE_ENTRIES],
    cq: [Completion; NVME_QUEUE_ENTRIES],
    sq_head: u16,
    sq_tail: u16,
    cq_head: u16,
    cq_phase: bool,
    next_cid: u16,
    /// Zero means free; otherwise this slot stores the exact outstanding CID.
    inflight: [u16; NVME_QUEUE_ENTRIES],
}
impl Queue {
    pub const fn new() -> Self {
        Self {
            sq: [Submission::EMPTY; NVME_QUEUE_ENTRIES],
            cq: [Completion::EMPTY; NVME_QUEUE_ENTRIES],
            sq_head: 0,
            sq_tail: 0,
            cq_head: 0,
            cq_phase: true,
            next_cid: 1,
            inflight: [0; NVME_QUEUE_ENTRIES],
        }
    }

    pub fn submit(&mut self, mut cmd: Submission) -> Result<u16, &'static str> {
        let next_tail = (self.sq_tail + 1) % NVME_QUEUE_ENTRIES as u16;
        if next_tail == self.sq_head {
            return Err("NVMe submission queue full");
        }
        let slot = self.inflight.iter().position(|cid| *cid == 0)
            .ok_or("NVMe inflight table full")?;

        let mut cid = self.next_cid.max(1);
        let start = cid;
        loop {
            if !self.inflight.contains(&cid) { break; }
            cid = cid.wrapping_add(1).max(1);
            if cid == start { return Err("NVMe CID space exhausted"); }
        }
        self.next_cid = cid.wrapping_add(1).max(1);
        cmd.cid = cid;
        self.sq[self.sq_tail as usize] = cmd;
        self.sq_tail = next_tail;
        self.inflight[slot] = cid;
        Ok(cid)
    }

    /// Consume one phase-valid CQE and retire the request identified by the CQE CID.
    pub fn complete(&mut self) -> Result<Option<Completion>, &'static str> {
        let c = self.cq[self.cq_head as usize];
        if c.phase() != self.cq_phase { return Ok(None); }
        if c.cid == 0 { return Err("NVMe completion has invalid CID 0"); }
        let slot = self.inflight.iter_mut().find(|cid| **cid == c.cid)
            .ok_or("NVMe completion CID is not inflight")?;
        *slot = 0;
        self.sq_head = c.sq_head % NVME_QUEUE_ENTRIES as u16;
        self.cq_head = (self.cq_head + 1) % NVME_QUEUE_ENTRIES as u16;
        if self.cq_head == 0 { self.cq_phase = !self.cq_phase; }
        Ok(Some(c))
    }

    pub fn abort_all(&mut self) {
        self.inflight = [0; NVME_QUEUE_ENTRIES];
        self.sq_head = 0;
        self.sq_tail = 0;
        self.cq_head = 0;
        self.cq_phase = true;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Namespace {
    pub id: u32,
    pub blocks: u64,
    pub block_size: u32,
    pub active: bool,
}
impl Namespace {
    pub const EMPTY: Self = Self { id: 0, blocks: 0, block_size: 0, active: false };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControllerState { Discovered, Disabled, AdminReady, IoReady, Resetting, Failed }

pub struct NvmeController {
    pub device: u64,
    pub mmio: u64,
    pub state: ControllerState,
    pub timeout_ms: u32,
    pub admin: Queue,
    pub io: Queue,
    pub namespaces: [Namespace; MAX_NVME_NAMESPACES],
}
impl NvmeController {
    pub fn from_device(d: &Device) -> Result<Self, &'static str> {
        if (d.class_code, d.subclass, d.programming_interface) != (1, 8, 2) {
            return Err("not NVMe");
        }
        let mmio = d.resources.iter().find_map(|r| {
            if let Resource::Mmio { base, .. } = r { Some(*base) } else { None }
        }).ok_or("NVMe BAR missing")?;
        if mmio & 0x3fff != 0 { return Err("NVMe BAR alignment invalid"); }
        Ok(Self {
            device: d.id.0,
            mmio,
            state: ControllerState::Discovered,
            timeout_ms: 5000,
            admin: Queue::new(),
            io: Queue::new(),
            namespaces: [Namespace::EMPTY; MAX_NVME_NAMESPACES],
        })
    }

    pub fn initialize(&mut self) -> Result<(), &'static str> {
        self.state = ControllerState::Disabled;
        self.admin = Queue::new();
        self.io = Queue::new();
        self.state = ControllerState::AdminReady;
        Ok(())
    }

    pub fn add_namespace(&mut self, id: u32, blocks: u64, block_size: u32) -> Result<(), &'static str> {
        if id == 0 || blocks == 0 || !block_size.is_power_of_two() || block_size < 512 {
            return Err("invalid NVMe namespace");
        }
        let s = self.namespaces.iter_mut().find(|n| !n.active).ok_or("namespace table full")?;
        *s = Namespace { id, blocks, block_size, active: true };
        self.state = ControllerState::IoReady;
        Ok(())
    }

    fn validate_data_prps(prp1: u64, prp2: u64, bytes: u64) -> Result<(), &'static str> {
        if bytes == 0 || prp1 == 0 { return Err("NVMe data command has no PRP1"); }
        let first_bytes = PAGE_SIZE - (prp1 & (PAGE_SIZE - 1));
        if bytes > first_bytes {
            if prp2 == 0 || prp2 & (PAGE_SIZE - 1) != 0 {
                return Err("NVMe PRP2/list must be page aligned");
            }
        }
        Ok(())
    }

    pub fn submit_io(
        &mut self,
        nsid: u32,
        op: BlockOperation,
        lba: u64,
        blocks: u32,
        prp1: u64,
        prp2: u64,
    ) -> Result<u16, &'static str> {
        if self.state != ControllerState::IoReady { return Err("NVMe I/O unavailable"); }
        let ns = self.namespaces.iter().find(|n| n.active && n.id == nsid)
            .ok_or("namespace missing")?;

        match op {
            BlockOperation::Flush => {
                if blocks != 0 || prp1 != 0 || prp2 != 0 {
                    return Err("NVMe flush must not carry data");
                }
                self.io.submit(Submission { opcode: 0x00, nsid, ..Submission::EMPTY })
            }
            BlockOperation::Read | BlockOperation::Write => {
                if blocks == 0 { return Err("NVMe data request is empty"); }
                if lba.checked_add(blocks as u64).ok_or("LBA overflow")? > ns.blocks {
                    return Err("I/O outside namespace");
                }
                let bytes = (blocks as u64).checked_mul(ns.block_size as u64)
                    .ok_or("NVMe byte length overflow")?;
                Self::validate_data_prps(prp1, prp2, bytes)?;
                let opcode = if op == BlockOperation::Read { 0x02 } else { 0x01 };
                self.io.submit(Submission {
                    opcode,
                    nsid,
                    prp1,
                    prp2,
                    cdw10: lba as u32,
                    cdw11: (lba >> 32) as u32,
                    cdw12: blocks - 1,
                    ..Submission::EMPTY
                })
            }
            BlockOperation::Discard => {
                // Dataset Management requires a DMA-resident range list. Refuse to emit a
                // malformed DSM command until that range-list allocator is wired in.
                Err("NVMe discard requires a DSM range-list buffer")
            }
        }
    }

    pub fn reset(&mut self) -> Result<(), &'static str> {
        self.state = ControllerState::Resetting;
        self.admin.abort_all();
        self.io.abort_all();
        self.state = ControllerState::Disabled;
        self.initialize()
    }
}
