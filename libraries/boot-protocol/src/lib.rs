#![no_std]

/// Version of the loader-to-kernel handoff contract.
pub const BOOT_PROTOCOL_VERSION: u32 = 14;
pub const BOOT_INFO_MAGIC: u64 = 0x5449_5441_4E56_3134; // "TITANV14"
pub const UEFI_PAGE_SIZE: u64 = 4096;
pub const MAX_BOOT_MODULES: usize = 8;
pub const BOOT_MODULE_NAME_BYTES: usize = 32;

pub mod boot_module_kind {
    pub const NONE: u32 = 0;
    pub const USER_ELF: u32 = 1;
    pub const DATA: u32 = 2;
    pub const BOOT_VOLUME: u32 = 3;
}

// BootInfo is currently stored in one loader-reserved 4 KiB page.
const _: [(); 1] = [(); (core::mem::size_of::<BootInfo>() <= UEFI_PAGE_SIZE as usize) as usize];

/// Immutable handoff data passed from the Titanweave UEFI loader to WeaveCore.
///
/// All addresses are numeric virtual or physical addresses rather than Rust
/// references. This keeps the ABI stable across the UEFI and bare-metal Rust
/// targets and makes address-space ownership explicit.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BootInfo {
    pub magic: u64,
    pub protocol_version: u32,
    pub structure_size: u32,
    pub memory_map: MemoryMapInfo,
    pub kernel: KernelImageInfo,
    pub bootstrap: BootstrapInfo,
    pub framebuffer: FramebufferInfo,
    pub acpi: AcpiInfo,
    pub smp: SmpBootstrapInfo,
    pub modules: BootModulesInfo,
}

impl BootInfo {
    #[must_use]
    pub const fn new(
        memory_map: MemoryMapInfo,
        kernel: KernelImageInfo,
        bootstrap: BootstrapInfo,
        framebuffer: FramebufferInfo,
        acpi: AcpiInfo,
        smp: SmpBootstrapInfo,
        modules: BootModulesInfo,
    ) -> Self {
        Self {
            magic: BOOT_INFO_MAGIC,
            protocol_version: BOOT_PROTOCOL_VERSION,
            structure_size: core::mem::size_of::<Self>() as u32,
            memory_map,
            kernel,
            bootstrap,
            framebuffer,
            acpi,
            smp,
            modules,
        }
    }

    #[must_use]
    pub const fn is_structurally_valid(&self) -> bool {
        self.magic == BOOT_INFO_MAGIC
            && self.protocol_version == BOOT_PROTOCOL_VERSION
            && self.structure_size as usize >= core::mem::size_of::<Self>()
            && self.memory_map.is_structurally_valid()
            && self.kernel.is_structurally_valid()
            && self.bootstrap.is_structurally_valid()
            && self.framebuffer.is_structurally_valid_or_empty()
            && self.acpi.is_structurally_valid()
            && self.smp.is_structurally_valid()
            && self.modules.is_structurally_valid()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MemoryMapInfo {
    pub buffer_address: u64,
    pub buffer_size: u64,
    pub descriptor_size: u64,
    pub descriptor_version: u32,
    pub descriptor_count: u32,
}

impl MemoryMapInfo {
    #[must_use]
    pub const fn is_structurally_valid(&self) -> bool {
        self.buffer_address != 0
            && self.buffer_size != 0
            && self.descriptor_size >= 40
            && self.descriptor_count != 0
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct KernelImageInfo {
    pub physical_base: u64,
    pub physical_size: u64,
    pub virtual_base: u64,
    pub virtual_size: u64,
    pub entry_point: u64,
}

impl KernelImageInfo {
    #[must_use]
    pub const fn is_structurally_valid(&self) -> bool {
        self.physical_base != 0
            && self.physical_size != 0
            && self.virtual_base != 0
            && self.virtual_size != 0
            && self.entry_point >= self.virtual_base
            && self.entry_point < self.virtual_base.saturating_add(self.virtual_size)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BootstrapInfo {
    pub page_table_root: u64,
    pub stack_physical_base: u64,
    pub stack_virtual_base: u64,
    pub stack_size: u64,
    pub identity_map_limit: u64,
}

impl BootstrapInfo {
    #[must_use]
    pub const fn is_structurally_valid(&self) -> bool {
        self.page_table_root != 0
            && self.stack_physical_base != 0
            && self.stack_virtual_base != 0
            && self.stack_size >= UEFI_PAGE_SIZE
            && self.identity_map_limit != 0
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FramebufferInfo {
    pub base_address: u64,
    pub byte_size: u64,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixel_format: u32,
}

pub mod framebuffer_pixel_format {
    /// UEFI PixelRedGreenBlueReserved8BitPerColor.
    pub const RGBX8888: u32 = 0;
    /// UEFI PixelBlueGreenRedReserved8BitPerColor.
    pub const BGRX8888: u32 = 1;
    /// UEFI PixelBitMask. K12 records it but the fallback renderer rejects it.
    pub const BITMASK: u32 = 2;
    /// UEFI PixelBltOnly. There is no post-ExitBootServices linear framebuffer.
    pub const BLT_ONLY: u32 = 3;
}

impl FramebufferInfo {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            base_address: 0,
            byte_size: 0,
            width: 0,
            height: 0,
            stride: 0,
            pixel_format: framebuffer_pixel_format::BLT_ONLY,
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.base_address == 0
    }

    #[must_use]
    pub const fn is_linear_32bpp(&self) -> bool {
        self.pixel_format == framebuffer_pixel_format::RGBX8888
            || self.pixel_format == framebuffer_pixel_format::BGRX8888
    }

    #[must_use]
    pub const fn is_structurally_valid_or_empty(&self) -> bool {
        if self.is_empty() {
            return self.byte_size == 0
                && self.width == 0
                && self.height == 0
                && self.stride == 0;
        }

        let row_bytes = (self.stride as u64).saturating_mul(4);
        let required_bytes = row_bytes.saturating_mul(self.height as u64);
        self.byte_size >= required_bytes
            && self.width != 0
            && self.height != 0
            && self.stride >= self.width
            && self.pixel_format <= framebuffer_pixel_format::BLT_ONLY
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AcpiInfo {
    /// Physical address of an ACPI 1.0 or 2.0+ RSDP supplied by UEFI.
    pub rsdp_address: u64,
}

impl AcpiInfo {
    #[must_use]
    pub const fn is_structurally_valid(&self) -> bool {
        self.rsdp_address != 0
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SmpBootstrapInfo {
    /// Loader-reserved page below 1 MiB used for the AP startup trampoline.
    pub trampoline_physical_base: u64,
    pub trampoline_size: u64,
    pub maximum_logical_cpus: u32,
    pub reserved: u32,
}

impl SmpBootstrapInfo {
    #[must_use]
    pub const fn is_structurally_valid(&self) -> bool {
        self.trampoline_physical_base != 0
            && self.trampoline_physical_base < 0x10_0000
            && self.trampoline_physical_base & (UEFI_PAGE_SIZE - 1) == 0
            && self.trampoline_size >= UEFI_PAGE_SIZE
            && self.maximum_logical_cpus != 0
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BootModuleInfo {
    pub name: [u8; BOOT_MODULE_NAME_BYTES],
    pub kind: u32,
    pub flags: u32,
    pub physical_address: u64,
    pub byte_size: u64,
    pub entry_hint: u64,
}

impl BootModuleInfo {
    pub const EMPTY: Self = Self {
        name: [0; BOOT_MODULE_NAME_BYTES],
        kind: boot_module_kind::NONE,
        flags: 0,
        physical_address: 0,
        byte_size: 0,
        entry_hint: 0,
    };

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.kind == boot_module_kind::NONE
    }

    #[must_use]
    pub const fn is_structurally_valid(&self) -> bool {
        self.is_empty() || (self.physical_address != 0 && self.byte_size != 0)
    }

    #[must_use]
    pub fn name_bytes(&self) -> &[u8] {
        let mut length = 0;
        while length < self.name.len() && self.name[length] != 0 {
            length += 1;
        }
        &self.name[..length]
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BootModulesInfo {
    pub count: u32,
    pub reserved: u32,
    pub entries: [BootModuleInfo; MAX_BOOT_MODULES],
}

impl BootModulesInfo {
    pub const fn empty() -> Self {
        Self {
            count: 0,
            reserved: 0,
            entries: [BootModuleInfo::EMPTY; MAX_BOOT_MODULES],
        }
    }

    #[must_use]
    pub const fn is_structurally_valid(&self) -> bool {
        if self.count as usize > MAX_BOOT_MODULES {
            return false;
        }
        let mut index = 0;
        while index < self.count as usize {
            if !self.entries[index].is_structurally_valid() || self.entries[index].is_empty() {
                return false;
            }
            index += 1;
        }
        true
    }

    pub fn push(&mut self, module: BootModuleInfo) -> Result<(), &'static str> {
        let index = self.count as usize;
        if index >= MAX_BOOT_MODULES {
            return Err("boot module table is full");
        }
        if !module.is_structurally_valid() || module.is_empty() {
            return Err("boot module descriptor is invalid");
        }
        self.entries[index] = module;
        self.count += 1;
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = &BootModuleInfo> {
        self.entries[..self.count as usize].iter()
    }
}

/// Stable 40-byte prefix of a UEFI memory descriptor.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct UefiMemoryDescriptorPrefix {
    pub memory_type: u32,
    pub padding: u32,
    pub physical_start: u64,
    pub virtual_start: u64,
    pub page_count: u64,
    pub attributes: u64,
}

pub mod uefi_memory_type {
    pub const RESERVED: u32 = 0;
    pub const LOADER_CODE: u32 = 1;
    pub const LOADER_DATA: u32 = 2;
    pub const BOOT_SERVICES_CODE: u32 = 3;
    pub const BOOT_SERVICES_DATA: u32 = 4;
    pub const RUNTIME_SERVICES_CODE: u32 = 5;
    pub const RUNTIME_SERVICES_DATA: u32 = 6;
    pub const CONVENTIONAL: u32 = 7;
    pub const UNUSABLE: u32 = 8;
    pub const ACPI_RECLAIM: u32 = 9;
    pub const ACPI_NON_VOLATILE: u32 = 10;
    pub const MMIO: u32 = 11;
    pub const MMIO_PORT_SPACE: u32 = 12;
    pub const PAL_CODE: u32 = 13;
    pub const PERSISTENT_MEMORY: u32 = 14;
}
