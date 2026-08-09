//! K13 backend-neutral GPU buffer-object and memory-domain policy.
//!
//! This is accounting/lifecycle infrastructure. Vendor backends later connect
//! these objects to real VRAM/GTT page tables without changing compositor or
//! userspace ownership semantics.

pub const MAX_GPU_BUFFERS: usize = 128;
pub const MAX_BUFFER_BYTES: u64 = 1 << 34; // 16 GiB per object safety ceiling.

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryDomain {
    System = 1,
    Gtt = 2,
    Vram = 3,
}

#[derive(Clone, Copy, Debug)]
pub struct BufferObject {
    pub id: u64,
    pub owner: u64,
    pub bytes: u64,
    pub alignment: u64,
    pub domain: MemoryDomain,
    pub generation: u32,
    pub pinned: bool,
    pub active: bool,
}

impl BufferObject {
    pub const EMPTY: Self = Self {
        id: 0,
        owner: 0,
        bytes: 0,
        alignment: 0,
        domain: MemoryDomain::System,
        generation: 0,
        pinned: false,
        active: false,
    };
}

pub struct GpuMemoryManager {
    buffers: [BufferObject; MAX_GPU_BUFFERS],
    next_id: u64,
    system_budget: u64,
    gtt_budget: u64,
    vram_budget: u64,
    system_used: u64,
    gtt_used: u64,
    vram_used: u64,
}

impl GpuMemoryManager {
    pub const fn new(system_budget: u64, gtt_budget: u64, vram_budget: u64) -> Self {
        Self {
            buffers: [BufferObject::EMPTY; MAX_GPU_BUFFERS],
            next_id: 1,
            system_budget,
            gtt_budget,
            vram_budget,
            system_used: 0,
            gtt_used: 0,
            vram_used: 0,
        }
    }

    pub fn create(
        &mut self,
        owner: u64,
        bytes: u64,
        alignment: u64,
        domain: MemoryDomain,
    ) -> Result<u64, &'static str> {
        if owner == 0 || bytes == 0 || bytes > MAX_BUFFER_BYTES {
            return Err("invalid GPU buffer request");
        }
        if alignment < 4096 || !alignment.is_power_of_two() {
            return Err("invalid GPU buffer alignment");
        }
        let (used, budget) = self.domain_usage(domain);
        if used.checked_add(bytes).ok_or("GPU buffer accounting overflow")? > budget {
            return Err("GPU memory-domain budget exceeded");
        }
        let index = self.buffers.iter().position(|entry| !entry.active).ok_or("GPU buffer table full")?;
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or("GPU buffer id exhausted")?;
        self.charge(domain, bytes)?;
        self.buffers[index] = BufferObject {
            id,
            owner,
            bytes,
            alignment,
            domain,
            generation: 1,
            pinned: false,
            active: true,
        };
        Ok(id)
    }

    pub fn migrate(&mut self, owner: u64, id: u64, new_domain: MemoryDomain) -> Result<(), &'static str> {
        let index = self.index_for(owner, id)?;
        if self.buffers[index].pinned {
            return Err("pinned GPU buffer cannot migrate");
        }
        let old_domain = self.buffers[index].domain;
        if old_domain == new_domain {
            return Ok(());
        }
        let bytes = self.buffers[index].bytes;
        let (used, budget) = self.domain_usage(new_domain);
        if used.checked_add(bytes).ok_or("GPU migration accounting overflow")? > budget {
            return Err("destination GPU memory-domain budget exceeded");
        }
        self.charge(new_domain, bytes)?;
        self.uncharge(old_domain, bytes)?;
        self.buffers[index].domain = new_domain;
        self.buffers[index].generation = self.buffers[index].generation.saturating_add(1);
        Ok(())
    }

    pub fn pin(&mut self, owner: u64, id: u64, pinned: bool) -> Result<(), &'static str> {
        let index = self.index_for(owner, id)?;
        self.buffers[index].pinned = pinned;
        Ok(())
    }

    pub fn destroy(&mut self, owner: u64, id: u64) -> Result<(), &'static str> {
        let index = self.index_for(owner, id)?;
        if self.buffers[index].pinned {
            return Err("pinned GPU buffer cannot be destroyed");
        }
        let entry = self.buffers[index];
        self.uncharge(entry.domain, entry.bytes)?;
        self.buffers[index] = BufferObject::EMPTY;
        Ok(())
    }

    #[must_use]
    pub fn active_count(&self) -> usize {
        self.buffers.iter().filter(|entry| entry.active).count()
    }

    #[must_use]
    pub const fn used_bytes(&self, domain: MemoryDomain) -> u64 {
        match domain {
            MemoryDomain::System => self.system_used,
            MemoryDomain::Gtt => self.gtt_used,
            MemoryDomain::Vram => self.vram_used,
        }
    }

    fn index_for(&self, owner: u64, id: u64) -> Result<usize, &'static str> {
        self.buffers
            .iter()
            .position(|entry| entry.active && entry.id == id && entry.owner == owner)
            .ok_or("GPU buffer not found or wrong owner")
    }

    const fn domain_usage(&self, domain: MemoryDomain) -> (u64, u64) {
        match domain {
            MemoryDomain::System => (self.system_used, self.system_budget),
            MemoryDomain::Gtt => (self.gtt_used, self.gtt_budget),
            MemoryDomain::Vram => (self.vram_used, self.vram_budget),
        }
    }

    fn charge(&mut self, domain: MemoryDomain, bytes: u64) -> Result<(), &'static str> {
        let slot = match domain {
            MemoryDomain::System => &mut self.system_used,
            MemoryDomain::Gtt => &mut self.gtt_used,
            MemoryDomain::Vram => &mut self.vram_used,
        };
        *slot = slot.checked_add(bytes).ok_or("GPU memory usage overflow")?;
        Ok(())
    }

    fn uncharge(&mut self, domain: MemoryDomain, bytes: u64) -> Result<(), &'static str> {
        let slot = match domain {
            MemoryDomain::System => &mut self.system_used,
            MemoryDomain::Gtt => &mut self.gtt_used,
            MemoryDomain::Vram => &mut self.vram_used,
        };
        *slot = slot.checked_sub(bytes).ok_or("GPU memory usage underflow")?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MemorySelfTestReport {
    pub created: usize,
    pub final_vram_bytes: u64,
    pub final_gtt_bytes: u64,
}

pub fn run_self_test() -> Result<MemorySelfTestReport, &'static str> {
    let mut manager = GpuMemoryManager::new(64 << 20, 64 << 20, 128 << 20);
    let scanout = manager.create(1, 8 << 20, 4096, MemoryDomain::Vram)?;
    let staging = manager.create(1, 4 << 20, 4096, MemoryDomain::System)?;
    manager.migrate(1, staging, MemoryDomain::Gtt)?;
    manager.pin(1, scanout, true)?;
    if manager.destroy(1, scanout).is_ok() {
        return Err("pinned GPU buffer destruction was not rejected");
    }
    manager.pin(1, scanout, false)?;
    manager.destroy(1, scanout)?;
    if manager.active_count() != 1 || manager.used_bytes(MemoryDomain::Gtt) != 4 << 20 {
        return Err("GPU memory lifecycle self-test failed");
    }
    Ok(MemorySelfTestReport {
        created: 2,
        final_vram_bytes: manager.used_bytes(MemoryDomain::Vram),
        final_gtt_bytes: manager.used_bytes(MemoryDomain::Gtt),
    })
}
