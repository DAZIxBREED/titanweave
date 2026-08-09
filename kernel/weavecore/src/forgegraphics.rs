//! ForgeGraphics K13 backend-neutral device contract.
//!
//! K13 preserves K12 semantics and does not pretend a firmware framebuffer is a real GPU driver.  It
//! establishes the contract that later AMD, Intel, NVIDIA, VirtIO-GPU, and
//! multi-GPU backends must implement without changing compositor semantics.

pub const FORGEGRAPHICS_ABI_VERSION: u32 = 1;
pub const MAX_GRAPHICS_ADAPTERS: usize = 16;

pub const CAP_SCANOUT: u64 = 1 << 0;
pub const CAP_BLIT: u64 = 1 << 1;
pub const CAP_COMPUTE: u64 = 1 << 2;
pub const CAP_TIMELINE_FENCE: u64 = 1 << 3;
pub const CAP_SHARED_MEMORY: u64 = 1 << 4;
pub const CAP_MULTI_GPU_COPY: u64 = 1 << 5;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendKind {
    FirmwareFramebuffer = 1,
    VirtioGpu = 2,
    AmdNative = 3,
    IntelNative = 4,
    NvidiaNative = 5,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AdapterInfo {
    pub adapter_id: u64,
    pub backend_kind: u32,
    pub flags: u32,
    pub capabilities: u64,
    pub dedicated_memory_bytes: u64,
    pub shared_memory_bytes: u64,
}

impl AdapterInfo {
    pub const EMPTY: Self = Self {
        adapter_id: 0,
        backend_kind: 0,
        flags: 0,
        capabilities: 0,
        dedicated_memory_bytes: 0,
        shared_memory_bytes: 0,
    };
}

pub struct AdapterRegistry {
    adapters: [AdapterInfo; MAX_GRAPHICS_ADAPTERS],
    count: usize,
}

impl AdapterRegistry {
    pub const fn new() -> Self {
        Self { adapters: [AdapterInfo::EMPTY; MAX_GRAPHICS_ADAPTERS], count: 0 }
    }

    pub fn register(&mut self, adapter: AdapterInfo) -> Result<(), &'static str> {
        if adapter.adapter_id == 0 || adapter.capabilities == 0 {
            return Err("invalid graphics adapter descriptor");
        }
        if self.adapters[..self.count].iter().any(|entry| entry.adapter_id == adapter.adapter_id) {
            return Err("graphics adapter id already registered");
        }
        if self.count == MAX_GRAPHICS_ADAPTERS {
            return Err("graphics adapter registry is full");
        }
        self.adapters[self.count] = adapter;
        self.count += 1;
        Ok(())
    }

    #[must_use]
    pub const fn count(&self) -> usize { self.count }

    #[must_use]
    pub fn primary_scanout(&self) -> Option<AdapterInfo> {
        self.adapters[..self.count]
            .iter()
            .copied()
            .find(|adapter| adapter.capabilities & CAP_SCANOUT != 0)
    }
}

pub fn run_self_test() -> Result<usize, &'static str> {
    let mut registry = AdapterRegistry::new();
    registry.register(AdapterInfo {
        adapter_id: 1,
        backend_kind: BackendKind::FirmwareFramebuffer as u32,
        flags: 0,
        capabilities: CAP_SCANOUT | CAP_BLIT,
        dedicated_memory_bytes: 0,
        shared_memory_bytes: 8 * 1024 * 1024,
    })?;
    registry.register(AdapterInfo {
        adapter_id: 2,
        backend_kind: BackendKind::VirtioGpu as u32,
        flags: 0,
        capabilities: CAP_SCANOUT | CAP_BLIT | CAP_TIMELINE_FENCE | CAP_SHARED_MEMORY,
        dedicated_memory_bytes: 128 * 1024 * 1024,
        shared_memory_bytes: 256 * 1024 * 1024,
    })?;
    if registry.primary_scanout().is_none() || registry.count() != 2 {
        return Err("ForgeGraphics registry self-test failed");
    }
    Ok(registry.count())
}
