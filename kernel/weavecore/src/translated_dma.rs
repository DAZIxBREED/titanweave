//! K14.B hardware-translated DMA qualification.
//!
//! K11 established default-deny policy and backend-neutral IOVA/domain models.
//! K14.B proves that policy against a real IOMMU translation engine.  The QEMU
//! qualification path uses Intel VT-d plus QEMU's EDU PCI DMA endpoint so the
//! test crosses the actual emulated IOMMU: an allowed IOVA round-trip must
//! succeed, then the same device is denied after its leaf PTE is revoked and
//! the IOTLB is invalidated.
//!
//! This module deliberately performs the live test in a short, quiescent
//! window and disables translation again before normal K13 userspace resumes.
//! K14.C will use the same page-table/ownership rules for a persistent native
//! GPU domain.  A successful K14.B result therefore means "hardware translation
//! qualified", not "every PCI device has already been migrated to translated
//! DMA".

use core::arch::asm;
use core::ptr;

use crate::{
    device::DeviceId,
    dma::DmaDirection,
    forgebus,
    k11_backends,
    memory::{FrameAllocator, FRAME_SIZE},
    paging,
    pci::{self, PciFunction},
    pci_address::{PciAddress, RequesterId},
    serial,
    sync::SpinLock,
};

const QEMU_EDU_VENDOR: u16 = 0x1234;
const QEMU_EDU_DEVICE: u16 = 0x11e8;
const EDU_BAR_BYTES: u64 = 0x1000;
const EDU_ID: u64 = 0x00;
const EDU_LIVENESS: u64 = 0x04;
const EDU_DMA_SRC: u64 = 0x80;
const EDU_DMA_DST: u64 = 0x88;
const EDU_DMA_COUNT: u64 = 0x90;
const EDU_DMA_CMD: u64 = 0x98;
const EDU_INTERNAL_BUFFER: u64 = 0x0004_0000;
const EDU_DMA_START: u64 = 1 << 0;
const EDU_DMA_FROM_EDU: u64 = 1 << 1;
const EDU_TEST_BYTES: u64 = 256;
const EDU_WAIT_SPINS: u64 = 20_000_000;

const VTD_MMIO_BYTES: u64 = 0x1_0000;
const VTD_REG_VERSION: u64 = 0x00;
const VTD_REG_CAP: u64 = 0x08;
const VTD_REG_ECAP: u64 = 0x10;
const VTD_REG_GCMD: u64 = 0x18;
const VTD_REG_GSTS: u64 = 0x1c;
const VTD_REG_RTADDR: u64 = 0x20;
const VTD_REG_CCMD: u64 = 0x28;
const VTD_REG_FSTS: u64 = 0x34;

const VTD_GCMD_TE: u32 = 1 << 31;
const VTD_GCMD_SRTP: u32 = 1 << 30;
const VTD_GSTS_TES: u32 = 1 << 31;
const VTD_GSTS_RTPS: u32 = 1 << 30;
const VTD_CCMD_ICC: u64 = 1 << 63;
const VTD_CCMD_CIRG_GLOBAL: u64 = 1 << 61;
const VTD_IOTLB_IVT: u64 = 1 << 63;
const VTD_IOTLB_IIRG_GLOBAL: u64 = 1 << 60;

const VTD_ROOT_PRESENT: u64 = 1 << 0;
const VTD_CONTEXT_PRESENT: u64 = 1 << 0;
const VTD_SL_READ: u64 = 1 << 0;
const VTD_SL_WRITE: u64 = 1 << 1;
const VTD_ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;
const VTD_DOMAIN_ID: u16 = 0x014b;
const TEST_IOVA_SOURCE: u64 = 0x0001_0000;
const TEST_IOVA_DEST: u64 = 0x0001_1000;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardwareIommuBackend {
    None = 0,
    IntelVtd = 1,
    AmdVi = 2,
}

#[derive(Clone, Copy, Debug)]
pub struct TranslationQualification {
    pub backend: HardwareIommuBackend,
    pub engine_present: bool,
    pub hardware_translated: bool,
    pub mappings_verified: u32,
    pub invalidations: u32,
    pub blocked_faults: u32,
    pub requester: u16,
    pub domain_id: u16,
}

impl TranslationQualification {
    pub const EMPTY: Self = Self {
        backend: HardwareIommuBackend::None,
        engine_present: false,
        hardware_translated: false,
        mappings_verified: 0,
        invalidations: 0,
        blocked_faults: 0,
        requester: 0,
        domain_id: 0,
    };
}

static STATE: SpinLock<TranslationQualification> = SpinLock::new(TranslationQualification::EMPTY);

#[derive(Clone, Copy)]
struct TablePages {
    root: u64,
    context: u64,
    level4: u64,
    level3: u64,
    level2: u64,
    level1: u64,
    levels: u8,
    aw: u64,
}

impl TablePages {
    fn allocate(allocator: &mut FrameAllocator<'_>, cap: u64) -> Result<Self, &'static str> {
        let sagaw = ((cap >> 8) & 0x1f) as u8;
        let (levels, aw) = if sagaw & (1 << 2) != 0 {
            (4u8, 2u64) // 48-bit adjusted guest address width.
        } else if sagaw & (1 << 1) != 0 {
            (3u8, 1u64) // 39-bit adjusted guest address width.
        } else {
            return Err("VT-d unit lacks 39/48-bit second-level translation");
        };
        let root = allocate_zeroed_page(allocator)?;
        let context = allocate_zeroed_page(allocator)?;
        let level4 = allocate_zeroed_page(allocator)?;
        let level3 = allocate_zeroed_page(allocator)?;
        let level2 = allocate_zeroed_page(allocator)?;
        let level1 = allocate_zeroed_page(allocator)?;
        Ok(Self { root, context, level4, level3, level2, level1, levels, aw })
    }

    fn slpt_root(self) -> u64 {
        if self.levels == 4 { self.level4 } else { self.level3 }
    }

    fn install_walk(self) {
        if self.levels == 4 {
            write_table_entry(self.level4, 0, self.level3 | VTD_SL_READ | VTD_SL_WRITE);
        }
        write_table_entry(self.level3, 0, self.level2 | VTD_SL_READ | VTD_SL_WRITE);
        write_table_entry(self.level2, 0, self.level1 | VTD_SL_READ | VTD_SL_WRITE);
        flush_page(self.level4);
        flush_page(self.level3);
        flush_page(self.level2);
    }

    fn install_root_context(self, requester: RequesterId, domain_id: u16) {
        let bus = (requester.0 >> 8) as usize;
        let devfn = (requester.0 & 0xff) as usize;
        write_u64_pair(
            self.root + (bus as u64) * 16,
            self.context | VTD_ROOT_PRESENT,
            0,
        );
        let context_low = self.slpt_root() | VTD_CONTEXT_PRESENT;
        let context_high = ((domain_id as u64) << 8) | self.aw;
        write_u64_pair(
            self.context + (devfn as u64) * 16,
            context_low,
            context_high,
        );
        flush_page(self.root);
        flush_page(self.context);
    }

    fn map_4k(self, iova: u64, physical: u64, read: bool, write: bool) -> Result<(), &'static str> {
        if iova & (FRAME_SIZE - 1) != 0 || physical & (FRAME_SIZE - 1) != 0 {
            return Err("VT-d mapping is not 4 KiB aligned");
        }
        if iova >= (1u64 << 39) {
            return Err("K14.B test IOVA exceeds 39-bit qualification window");
        }
        let l3 = ((iova >> 30) & 0x1ff) as usize;
        let l2 = ((iova >> 21) & 0x1ff) as usize;
        let l1 = ((iova >> 12) & 0x1ff) as usize;
        if l3 != 0 || l2 != 0 {
            return Err("K14.B fixed qualification tables only cover the first 2 MiB IOVA window");
        }
        let mut entry = physical & VTD_ADDRESS_MASK;
        if read { entry |= VTD_SL_READ; }
        if write { entry |= VTD_SL_WRITE; }
        if entry & (VTD_SL_READ | VTD_SL_WRITE) == 0 {
            return Err("VT-d mapping has no DMA permissions");
        }
        write_table_entry(self.level1, l1, entry);
        flush_cache_line(self.level1 + (l1 as u64) * 8);
        Ok(())
    }

    fn unmap_4k(self, iova: u64) -> Result<(), &'static str> {
        if iova & (FRAME_SIZE - 1) != 0 || iova >= (1u64 << 39) {
            return Err("invalid VT-d unmap IOVA");
        }
        let l1 = ((iova >> 12) & 0x1ff) as usize;
        write_table_entry(self.level1, l1, 0);
        flush_cache_line(self.level1 + (l1 as u64) * 8);
        Ok(())
    }

    fn clear_context(self, requester: RequesterId) {
        let bus = (requester.0 >> 8) as usize;
        let devfn = (requester.0 & 0xff) as usize;
        write_u64_pair(self.context + (devfn as u64) * 16, 0, 0);
        write_u64_pair(self.root + (bus as u64) * 16, 0, 0);
        flush_page(self.context);
        flush_page(self.root);
    }

    fn release(self, allocator: &mut FrameAllocator<'_>) {
        for page in [self.root, self.context, self.level4, self.level3, self.level2, self.level1] {
            let _ = allocator.deallocate_frame(page);
        }
    }
}

struct IntelVtdHardware {
    mmio: u64,
    cap: u64,
    ecap: u64,
    iro: u64,
    gcmd_shadow: u32,
    invalidations: u32,
}

impl IntelVtdHardware {
    fn map(
        allocator: &mut FrameAllocator<'_>,
        kernel_cr3: u64,
        register_base: u64,
    ) -> Result<Self, &'static str> {
        let mmio = paging::map_kernel_mmio(allocator, kernel_cr3, register_base, VTD_MMIO_BYTES)?;
        let version = read32(mmio + VTD_REG_VERSION);
        let cap = read64(mmio + VTD_REG_CAP);
        let ecap = read64(mmio + VTD_REG_ECAP);
        let iro = ((ecap >> 8) & 0x3ff) * 16;
        let iotlb_outside_aperture = match iro.checked_add(16) {
            Some(end) => end > VTD_MMIO_BYTES,
            None => true,
        };
        if iro == 0 || iotlb_outside_aperture {
            return Err("VT-d IOTLB register offset lies outside mapped aperture");
        }
        if read32(mmio + VTD_REG_GSTS) & VTD_GSTS_TES != 0 {
            return Err("VT-d translation was already enabled before K14.B qualification");
        }
        serial::println(format_args!(
            "[IOMH] Intel VT-d hardware engine: version={}.{} cap={:#018x} ecap={:#018x} iro={:#x}",
            version >> 4,
            version & 0xf,
            cap,
            ecap,
            iro
        ));
        Ok(Self { mmio, cap, ecap, iro, gcmd_shadow: 0, invalidations: 0 })
    }

    fn set_root_table(&mut self, root: u64) -> Result<(), &'static str> {
        write64(self.mmio + VTD_REG_RTADDR, root & VTD_ADDRESS_MASK);
        self.gcmd_shadow |= VTD_GCMD_SRTP;
        write32(self.mmio + VTD_REG_GCMD, self.gcmd_shadow);
        wait32(self.mmio + VTD_REG_GSTS, VTD_GSTS_RTPS, true, "VT-d set-root-pointer timeout")?;
        self.gcmd_shadow &= !VTD_GCMD_SRTP;
        self.invalidate_context_global()?;
        self.invalidate_iotlb_global()?;
        Ok(())
    }

    fn enable_translation(&mut self) -> Result<(), &'static str> {
        self.gcmd_shadow |= VTD_GCMD_TE;
        write32(self.mmio + VTD_REG_GCMD, self.gcmd_shadow);
        wait32(self.mmio + VTD_REG_GSTS, VTD_GSTS_TES, true, "VT-d translation enable timeout")
    }

    fn disable_translation(&mut self) -> Result<(), &'static str> {
        self.gcmd_shadow &= !VTD_GCMD_TE;
        write32(self.mmio + VTD_REG_GCMD, self.gcmd_shadow);
        wait32(self.mmio + VTD_REG_GSTS, VTD_GSTS_TES, false, "VT-d translation disable timeout")
    }

    fn invalidate_context_global(&mut self) -> Result<(), &'static str> {
        write64(self.mmio + VTD_REG_CCMD, VTD_CCMD_ICC | VTD_CCMD_CIRG_GLOBAL);
        wait64(self.mmio + VTD_REG_CCMD, VTD_CCMD_ICC, false, "VT-d context invalidation timeout")?;
        self.invalidations = self.invalidations.saturating_add(1);
        Ok(())
    }

    fn invalidate_iotlb_global(&mut self) -> Result<(), &'static str> {
        let iotlb = self.mmio + self.iro + 8;
        write64(iotlb, VTD_IOTLB_IVT | VTD_IOTLB_IIRG_GLOBAL);
        wait64(iotlb, VTD_IOTLB_IVT, false, "VT-d IOTLB invalidation timeout")?;
        self.invalidations = self.invalidations.saturating_add(1);
        Ok(())
    }

    fn clear_faults(&self) {
        let status = read32(self.mmio + VTD_REG_FSTS);
        if status != 0 { write32(self.mmio + VTD_REG_FSTS, status); }
    }

    fn fault_status(&self) -> u32 { read32(self.mmio + VTD_REG_FSTS) }
}

#[must_use]
pub fn state() -> TranslationQualification { *STATE.lock() }


#[must_use]
pub fn hardware_translation_qualified() -> bool { state().hardware_translated }

/// K14.B live qualification entry.  Absence of the QEMU EDU endpoint is not a
/// fatal OS condition; it simply leaves hardware translation unqualified so the
/// native-GPU admission gate remains fail-closed.  The K14.B QEMU checker, on
/// the other hand, requires the endpoint and therefore cannot pass accidentally.
pub fn initialize_qualification(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
) -> Result<TranslationQualification, &'static str> {
    let active = k11_backends::active_iommu();
    let backend = match active {
        k11_backends::ActiveIommu::IntelVtd => HardwareIommuBackend::IntelVtd,
        k11_backends::ActiveIommu::AmdVi => HardwareIommuBackend::AmdVi,
        k11_backends::ActiveIommu::None => HardwareIommuBackend::None,
    };
    serial::println(format_args!(
        "[IOM2] K14.B translated-DMA foundation: discovered_backend={:?} default_deny=true",
        backend
    ));

    if backend == HardwareIommuBackend::AmdVi {
        // AMD-Vi table-format rules are retained in K11's backend-neutral core,
        // but this QEMU-qualified K14.B slice does not claim live AMD hardware
        // programming without a real IVRS platform to verify it against.
        serial::println(format_args!(
            "[AMDQ] AMD-Vi live translation qualification deferred to IVRS bare-metal target; native DMA remains fenced"
        ));
        let report = TranslationQualification { backend, engine_present: true, ..TranslationQualification::EMPTY };
        *STATE.lock() = report;
        return Ok(report);
    }
    if backend != HardwareIommuBackend::IntelVtd {
        let report = TranslationQualification::EMPTY;
        *STATE.lock() = report;
        return Ok(report);
    }

    let Some(edu) = find_edu() else {
        serial::println(format_args!(
            "[DMAT] no QEMU EDU DMA endpoint; live VT-d translation proof deferred and native DMA remains fenced"
        ));
        let report = TranslationQualification { backend, engine_present: true, ..TranslationQualification::EMPTY };
        *STATE.lock() = report;
        return Ok(report);
    };

    let report = qualify_intel_vtd_with_edu(allocator, kernel_cr3, edu)?;
    *STATE.lock() = report;
    Ok(report)
}

fn qualify_intel_vtd_with_edu(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
    edu: PciFunction,
) -> Result<TranslationQualification, &'static str> {
    let catalog_backend = k11_backends::intel_primary_register_base().ok_or("VT-d DRHD register base unavailable")?;
    let mut vtd = IntelVtdHardware::map(allocator, kernel_cr3, catalog_backend)?;
    let tables = TablePages::allocate(allocator, vtd.cap)?;
    tables.install_walk();

    let requester = PciAddress::new(0, edu.bus, edu.device, edu.function)?.requester_id();
    tables.install_root_context(requester, VTD_DOMAIN_ID);

    let (device_id, _) = forgebus::claim_pci_function(edu, b"titan-iommu-test", 0)?;
    forgebus::establish_dma_domain(device_id, 64, true)?;
    let source = forgebus::allocate_dma(allocator, device_id, FRAME_SIZE, DmaDirection::Bidirectional)?;
    let destination = match forgebus::allocate_dma(allocator, device_id, FRAME_SIZE, DmaDirection::Bidirectional) {
        Ok(mapping) => mapping,
        Err(error) => {
            let _ = forgebus::release_dma(allocator, device_id, source.physical);
            tables.release(allocator);
            return Err(error);
        }
    };

    fill_source(source.physical, destination.physical);
    tables.map_4k(TEST_IOVA_SOURCE, source.physical, true, false)?;
    tables.map_4k(TEST_IOVA_DEST, destination.physical, false, true)?;

    let Some(edu_bar) = pci::memory_bar_base(edu, 0) else {
        return cleanup_error(allocator, device_id, source.physical, destination.physical, tables, "QEMU EDU BAR0 missing");
    };
    let edu_mmio = paging::map_kernel_mmio(allocator, kernel_cr3, edu_bar, EDU_BAR_BYTES)?;
    pci::enable_memory_decode(edu);
    let edu_id = read32(edu_mmio + EDU_ID);
    let liveness_seed = 0x14b0_2026u32;
    write32(edu_mmio + EDU_LIVENESS, liveness_seed);
    if read32(edu_mmio + EDU_LIVENESS) != !liveness_seed {
        return cleanup_error(allocator, device_id, source.physical, destination.physical, tables, "QEMU EDU liveness register failed");
    }

    vtd.set_root_table(tables.root)?;
    vtd.clear_faults();
    vtd.enable_translation()?;
    pci::enable_bus_master(edu);

    serial::println(format_args!(
        "[IOVA] translated DMA map: requester={:#06x} domain={} source_iova={:#x}->{:#x} dest_iova={:#x}->{:#x} levels={}",
        requester.0,
        VTD_DOMAIN_ID,
        TEST_IOVA_SOURCE,
        source.physical,
        TEST_IOVA_DEST,
        destination.physical,
        tables.levels
    ));

    edu_dma(edu_mmio, TEST_IOVA_SOURCE, EDU_INTERNAL_BUFFER, EDU_TEST_BYTES, false)?;
    edu_dma(edu_mmio, EDU_INTERNAL_BUFFER, TEST_IOVA_DEST, EDU_TEST_BYTES, true)?;
    if !buffers_match(source.physical, destination.physical, EDU_TEST_BYTES as usize) {
        pci::disable_bus_master(edu);
        let _ = vtd.disable_translation();
        return cleanup_error(allocator, device_id, source.physical, destination.physical, tables, "translated EDU DMA round-trip mismatch");
    }
    serial::println(format_args!(
        "[DMAT] EDU translated DMA round-trip verified: id={:#010x} bytes={} requester={:#06x}",
        edu_id,
        EDU_TEST_BYTES,
        requester.0
    ));

    tables.unmap_4k(TEST_IOVA_DEST)?;
    vtd.invalidate_iotlb_global()?;
    zero_page(destination.physical);
    vtd.clear_faults();
    edu_dma(edu_mmio, EDU_INTERNAL_BUFFER, TEST_IOVA_DEST, EDU_TEST_BYTES, true)?;
    let denied = buffer_is_zero(destination.physical, EDU_TEST_BYTES as usize);
    let fault_status = vtd.fault_status();
    if !denied {
        pci::disable_bus_master(edu);
        let _ = vtd.disable_translation();
        return cleanup_error(allocator, device_id, source.physical, destination.physical, tables, "revoked IOVA remained writable by DMA device");
    }
    serial::println(format_args!(
        "[IOPF] unmapped DMA denied: requester={:#06x} iova={:#x} destination_unchanged=true fault_status={:#010x}",
        requester.0,
        TEST_IOVA_DEST,
        fault_status
    ));

    pci::disable_bus_master(edu);
    tables.clear_context(requester);
    vtd.invalidate_context_global()?;
    vtd.invalidate_iotlb_global()?;
    vtd.disable_translation()?;
    vtd.clear_faults();
    serial::println(format_args!(
        "[INVL] VT-d context/IOTLB invalidation verified: operations={} translation_enabled=false",
        vtd.invalidations
    ));

    let _ = forgebus::release_dma(allocator, device_id, source.physical);
    let _ = forgebus::release_dma(allocator, device_id, destination.physical);
    tables.release(allocator);
    serial::println(format_args!(
        "[REVK] translated DMA test domain revoked: device={} requester={:#06x} bus_master=false",
        device_id.0,
        requester.0
    ));

    let report = TranslationQualification {
        backend: HardwareIommuBackend::IntelVtd,
        engine_present: true,
        hardware_translated: true,
        mappings_verified: 2,
        invalidations: vtd.invalidations,
        blocked_faults: 1,
        requester: requester.0,
        domain_id: VTD_DOMAIN_ID,
    };
    serial::println(format_args!(
        "[IOMR] hardware translation qualification: backend=IntelVtd translated=true mappings={} blocked_faults={} invalidations={}",
        report.mappings_verified,
        report.blocked_faults,
        report.invalidations
    ));
    Ok(report)
}

fn cleanup_error(
    allocator: &mut FrameAllocator<'_>,
    device: DeviceId,
    source: u64,
    destination: u64,
    tables: TablePages,
    error: &'static str,
) -> Result<TranslationQualification, &'static str> {
    let _ = forgebus::release_dma(allocator, device, source);
    let _ = forgebus::release_dma(allocator, device, destination);
    tables.release(allocator);
    Err(error)
}

#[derive(Clone, Copy, Debug)]
pub struct PersistentDomainQualification {
    pub backend: HardwareIommuBackend,
    pub hardware_translated: bool,
    pub requester: u16,
    pub domain_id: u16,
    pub epochs: u32,
    pub mappings_retained: u32,
    pub invalidations: u32,
    pub revoked: bool,
}

impl PersistentDomainQualification {
    pub const EMPTY: Self = Self {
        backend: HardwareIommuBackend::None, hardware_translated: false, requester: 0,
        domain_id: 0, epochs: 0, mappings_retained: 0, invalidations: 0, revoked: false,
    };
}

/// K14.C2 persistent-domain surrogate proof. QEMU has no native Radeon model,
/// so the EDU endpoint is held behind one VT-d context and the same two IOVA
/// mappings for several DMA epochs before the domain is explicitly revoked.
/// This proves retention/lifecycle mechanics without pretending the EDU device
/// is an AMD GPU or leaving translation active for K13's iommu_platform=off path.
pub fn qualify_persistent_domain_surrogate(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
) -> Result<PersistentDomainQualification, &'static str> {
    if k11_backends::active_iommu() != k11_backends::ActiveIommu::IntelVtd {
        return Ok(PersistentDomainQualification::EMPTY);
    }
    let Some(edu) = find_edu() else { return Ok(PersistentDomainQualification::EMPTY); };
    let register_base = k11_backends::intel_primary_register_base().ok_or("VT-d DRHD register base unavailable")?;
    let mut vtd = IntelVtdHardware::map(allocator, kernel_cr3, register_base)?;
    let tables = TablePages::allocate(allocator, vtd.cap)?;
    tables.install_walk();
    let requester = PciAddress::new(0, edu.bus, edu.device, edu.function)?.requester_id();
    const DOMAIN: u16 = 0x14c2;
    const SRC_IOVA: u64 = 0x0001_4000;
    const DST_IOVA: u64 = 0x0001_5000;
    tables.install_root_context(requester, DOMAIN);

    let (device_id, _) = forgebus::claim_pci_function(edu, b"titan-iommu-test", 0)?;
    forgebus::establish_dma_domain(device_id, 64, true)?;
    let source = forgebus::allocate_dma(allocator, device_id, FRAME_SIZE, DmaDirection::Bidirectional)?;
    let destination = match forgebus::allocate_dma(allocator, device_id, FRAME_SIZE, DmaDirection::Bidirectional) {
        Ok(v) => v,
        Err(e) => { let _ = forgebus::release_dma(allocator, device_id, source.physical); tables.release(allocator); return Err(e); }
    };
    tables.map_4k(SRC_IOVA, source.physical, true, false)?;
    tables.map_4k(DST_IOVA, destination.physical, false, true)?;
    let Some(edu_bar) = pci::memory_bar_base(edu, 0) else {
        return cleanup_persistent_error(allocator, device_id, source.physical, destination.physical, tables, "QEMU EDU BAR0 missing");
    };
    let edu_mmio = paging::map_kernel_mmio(allocator, kernel_cr3, edu_bar, EDU_BAR_BYTES)?;
    pci::disable_bus_master(edu);
    pci::enable_memory_decode(edu);
    vtd.set_root_table(tables.root)?;
    vtd.clear_faults();
    vtd.enable_translation()?;
    pci::enable_bus_master(edu);

    let mut epochs = 0u32;
    for epoch in 0..3u32 {
        fill_source(source.physical, destination.physical);
        unsafe { ptr::write_volatile(source.physical as *mut u32, 0x14c2_0000 ^ epoch); }
        edu_dma(edu_mmio, SRC_IOVA, EDU_INTERNAL_BUFFER, EDU_TEST_BYTES, false)?;
        edu_dma(edu_mmio, EDU_INTERNAL_BUFFER, DST_IOVA, EDU_TEST_BYTES, true)?;
        if !buffers_match(source.physical, destination.physical, EDU_TEST_BYTES as usize) {
            pci::disable_bus_master(edu); let _ = vtd.disable_translation();
            return cleanup_persistent_error(allocator, device_id, source.physical, destination.physical, tables, "persistent-domain DMA epoch mismatch");
        }
        epochs = epochs.saturating_add(1);
    }
    serial::println(format_args!(
        "[PDOM] K14.C2 persistent translated-domain surrogate: backend=IntelVtd requester={:#06x} domain={} epochs={} mappings_retained=2 bus_master=true",
        requester.0, DOMAIN, epochs
    ));

    pci::disable_bus_master(edu);
    tables.clear_context(requester);
    vtd.invalidate_context_global()?;
    vtd.invalidate_iotlb_global()?;
    vtd.disable_translation()?;
    vtd.clear_faults();
    let invalidations = vtd.invalidations;
    let _ = forgebus::release_dma(allocator, device_id, source.physical);
    let _ = forgebus::release_dma(allocator, device_id, destination.physical);
    tables.release(allocator);
    serial::println(format_args!(
        "[PDRV] K14.C2 persistent-domain surrogate revoked: requester={:#06x} domain={} bus_master=false translation_enabled=false",
        requester.0, DOMAIN
    ));
    Ok(PersistentDomainQualification {
        backend: HardwareIommuBackend::IntelVtd, hardware_translated: true, requester: requester.0,
        domain_id: DOMAIN, epochs, mappings_retained: 2, invalidations, revoked: true,
    })
}

fn cleanup_persistent_error(
    allocator: &mut FrameAllocator<'_>, device: DeviceId, source: u64, destination: u64,
    tables: TablePages, error: &'static str,
) -> Result<PersistentDomainQualification, &'static str> {
    let _ = forgebus::release_dma(allocator, device, source);
    let _ = forgebus::release_dma(allocator, device, destination);
    tables.release(allocator);
    Err(error)
}

fn find_edu() -> Option<PciFunction> {
    pci::find_first(|function| function.vendor_id == QEMU_EDU_VENDOR && function.device_id == QEMU_EDU_DEVICE)
}

fn edu_dma(mmio: u64, source: u64, destination: u64, count: u64, from_edu: bool) -> Result<(), &'static str> {
    write64(mmio + EDU_DMA_SRC, source);
    write64(mmio + EDU_DMA_DST, destination);
    write64(mmio + EDU_DMA_COUNT, count);
    let command = EDU_DMA_START | if from_edu { EDU_DMA_FROM_EDU } else { 0 };
    write64(mmio + EDU_DMA_CMD, command);
    for _ in 0..EDU_WAIT_SPINS {
        if read64(mmio + EDU_DMA_CMD) & EDU_DMA_START == 0 { return Ok(()); }
        core::hint::spin_loop();
    }
    Err("QEMU EDU DMA timed out")
}

fn allocate_zeroed_page(allocator: &mut FrameAllocator<'_>) -> Result<u64, &'static str> {
    let page = allocator.allocate_frame().ok_or("IOMMU page-table allocation failed")?;
    unsafe { ptr::write_bytes(page as *mut u8, 0, FRAME_SIZE as usize) };
    flush_page(page);
    Ok(page)
}

fn write_table_entry(table: u64, index: usize, value: u64) {
    unsafe { ptr::write_volatile((table as *mut u64).add(index), value) };
}

fn write_u64_pair(address: u64, low: u64, high: u64) {
    unsafe {
        ptr::write_volatile(address as *mut u64, low);
        ptr::write_volatile((address + 8) as *mut u64, high);
    }
}

fn flush_page(page: u64) {
    let mut offset = 0u64;
    while offset < FRAME_SIZE {
        flush_cache_line(page + offset);
        offset += 64;
    }
    unsafe { asm!("mfence", options(nostack, preserves_flags)) };
}

fn flush_cache_line(address: u64) {
    unsafe { asm!("clflush [{}]", in(reg) address, options(nostack, preserves_flags)) };
}

fn fill_source(source: u64, destination: u64) {
    zero_page(source);
    zero_page(destination);
    for index in 0..(EDU_TEST_BYTES as usize / 4) {
        let value = 0x5457_0000u32 ^ (index as u32).wrapping_mul(0x0101_0101);
        unsafe { ptr::write_volatile((source as *mut u32).add(index), value) };
    }
}

fn zero_page(address: u64) {
    unsafe { ptr::write_bytes(address as *mut u8, 0, FRAME_SIZE as usize) };
}

fn buffers_match(a: u64, b: u64, bytes: usize) -> bool {
    for index in 0..bytes {
        let av = unsafe { ptr::read_volatile((a as *const u8).add(index)) };
        let bv = unsafe { ptr::read_volatile((b as *const u8).add(index)) };
        if av != bv { return false; }
    }
    true
}

fn buffer_is_zero(address: u64, bytes: usize) -> bool {
    for index in 0..bytes {
        if unsafe { ptr::read_volatile((address as *const u8).add(index)) } != 0 { return false; }
    }
    true
}

fn wait32(address: u64, mask: u32, set: bool, error: &'static str) -> Result<(), &'static str> {
    for _ in 0..EDU_WAIT_SPINS {
        let value = read32(address);
        if (value & mask != 0) == set { return Ok(()); }
        core::hint::spin_loop();
    }
    Err(error)
}

fn wait64(address: u64, mask: u64, set: bool, error: &'static str) -> Result<(), &'static str> {
    for _ in 0..EDU_WAIT_SPINS {
        let value = read64(address);
        if (value & mask != 0) == set { return Ok(()); }
        core::hint::spin_loop();
    }
    Err(error)
}

fn read32(address: u64) -> u32 { unsafe { ptr::read_volatile(address as *const u32) } }
fn read64(address: u64) -> u64 { unsafe { ptr::read_volatile(address as *const u64) } }
fn write32(address: u64, value: u32) { unsafe { ptr::write_volatile(address as *mut u32, value) } }
fn write64(address: u64, value: u64) { unsafe { ptr::write_volatile(address as *mut u64, value) } }
