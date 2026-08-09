//! K13.B VirtIO-GPU modern PCI transport.
//!
//! K13.A stopped at side-effect-free discovery. K13.B claims the adapter
//! through ForgeBus, establishes a bounded DMA ownership domain, walks the
//! modern VirtIO PCI capabilities, negotiates VERSION_1, creates split
//! control/cursor virtqueues, and proves the 2D transport with a real scanout
//! resource. The K12 GOP framebuffer remains the recovery path.
//!
//! Important current boundary: ForgeBus/DmaManager tracks and revokes every DMA
//! page used here. K11's hardware-IOMMU backends are not yet a complete page-
//! table programming implementation, so K13.B does not negotiate
//! VIRTIO_F_ACCESS_PLATFORM. Hardware-translated DMA remains a later hardening
//! gate rather than being falsely claimed here.

use core::mem::size_of;
use core::ptr;
use core::sync::atomic::{compiler_fence, Ordering};

use crate::{
    device::DeviceId,
    dma::DmaDirection,
    forgebus,
    memory::{FrameAllocator, FRAME_SIZE},
    paging,
    pci::{self, PciFunction},
    serial,
};
use crate::gpu_present::{DamageRect, PRESENT_BUFFER_COUNT};
use crate::gpu_topology::{VENDOR_VIRTIO, VIRTIO_GPU_MODERN_DEVICE};
use crate::sync::SpinLock;

const PCI_STATUS_CAP_LIST: u16 = 1 << 4;
const PCI_CAPABILITY_LIST: u8 = 0x34;
const PCI_CAP_ID_VENDOR: u8 = 0x09;

const VIRTIO_PCI_CAP_COMMON_CFG: u8 = 1;
const VIRTIO_PCI_CAP_NOTIFY_CFG: u8 = 2;
const VIRTIO_PCI_CAP_ISR_CFG: u8 = 3;
const VIRTIO_PCI_CAP_DEVICE_CFG: u8 = 4;

const COMMON_DFSELECT: u64 = 0;
const COMMON_DF: u64 = 4;
const COMMON_GFSELECT: u64 = 8;
const COMMON_GF: u64 = 12;
const COMMON_NUMQ: u64 = 18;
const COMMON_STATUS: u64 = 20;
const COMMON_Q_SELECT: u64 = 22;
const COMMON_Q_SIZE: u64 = 24;
const COMMON_Q_MSIX: u64 = 26;
const COMMON_Q_ENABLE: u64 = 28;
const COMMON_Q_NOFF: u64 = 30;
const COMMON_Q_DESCLO: u64 = 32;
const COMMON_Q_DESCHI: u64 = 36;
const COMMON_Q_AVAILLO: u64 = 40;
const COMMON_Q_AVAILHI: u64 = 44;
const COMMON_Q_USEDLO: u64 = 48;
const COMMON_Q_USEDHI: u64 = 52;

const STATUS_ACKNOWLEDGE: u8 = 1;
const STATUS_DRIVER: u8 = 2;
const STATUS_DRIVER_OK: u8 = 4;
const STATUS_FEATURES_OK: u8 = 8;
const STATUS_FAILED: u8 = 128;

const VIRTIO_F_VERSION_1_HIGH_BIT: u32 = 1 << 0; // feature bit 32
const VIRTIO_F_ACCESS_PLATFORM_HIGH_BIT: u32 = 1 << 1; // feature bit 33

const CONTROL_QUEUE_INDEX: u16 = 0;
const CURSOR_QUEUE_INDEX: u16 = 1;
const MAX_QUEUE_SIZE: u16 = 64;
const QUEUE_MINIMUM: u16 = 8;
const DESC_NEXT: u16 = 1;
const DESC_WRITE: u16 = 2;

const VIRTIO_GPU_CMD_GET_DISPLAY_INFO: u32 = 0x0100;
const VIRTIO_GPU_CMD_RESOURCE_CREATE_2D: u32 = 0x0101;
const VIRTIO_GPU_CMD_SET_SCANOUT: u32 = 0x0103;
const VIRTIO_GPU_CMD_RESOURCE_FLUSH: u32 = 0x0104;
const VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D: u32 = 0x0105;
const VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING: u32 = 0x0106;
const VIRTIO_GPU_RESP_OK_NODATA: u32 = 0x1100;
const VIRTIO_GPU_RESP_OK_DISPLAY_INFO: u32 = 0x1101;
const VIRTIO_GPU_FORMAT_B8G8R8X8_UNORM: u32 = 2;
const VIRTIO_GPU_MAX_SCANOUTS: usize = 16;
const VIRTIO_GPU_FLAG_FENCE: u32 = 1 << 0;

const RESOURCE_ID: u32 = 1;
const WORKSPACE_BYTES: u64 = FRAME_SIZE;
const WORKSPACE_RESPONSE_OFFSET: u64 = 2048;
const MAX_SCANOUT_WIDTH: u32 = 1920;
const MAX_SCANOUT_HEIGHT: u32 = 1080;
const COMMAND_TIMEOUT_SPINS: u64 = 50_000_000;

#[derive(Clone, Copy, Debug, Default)]
pub struct VirtioGpuProbe {
    pub present: bool,
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub device_id: u16,
    pub memory_bars: usize,
    pub bus_master_enabled: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct VirtioGpuTransportReport {
    pub device_id: DeviceId,
    pub driver_id: u64,
    pub control_queue_size: u16,
    pub cursor_queue_size: u16,
    pub scanout_id: u32,
    pub width: u32,
    pub height: u32,
    pub framebuffer_bytes: u64,
    pub negotiated_features_low: u32,
    pub negotiated_features_high: u32,
    pub device_scanouts: u32,
    pub transport_ready: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct VirtioGpuPresentationReport {
    pub buffers: u32,
    pub frames_presented: u64,
    pub damage_uploads: u64,
    pub last_fence: u64,
    pub front_resource: u32,
    pub fallback_armed: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct VirtioGpuPresentResult {
    pub frame_sequence: u64,
    pub fence_id: u64,
    pub resource_id: u32,
    pub damage: DamageRect,
}

#[derive(Clone, Copy, Debug)]
pub struct VirtioGpuRecoveryReport {
    pub device_id: DeviceId,
    pub frame_sequence: u64,
    pub completed_fence: u64,
    pub bus_master_enabled: bool,
    pub driver_ok: bool,
}

#[derive(Clone, Copy)]
struct MmioRegion {
    base: u64,
    length: u64,
}

impl MmioRegion {
    const EMPTY: Self = Self { base: 0, length: 0 };
    fn contains(self, offset: u64, bytes: u64) -> bool {
        offset.checked_add(bytes).is_some_and(|end| end <= self.length)
    }
}

#[derive(Clone, Copy)]
struct ModernCapabilities {
    common: MmioRegion,
    notify: MmioRegion,
    isr: MmioRegion,
    device: MmioRegion,
    notify_multiplier: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Descriptor {
    address: u64,
    length: u32,
    flags: u16,
    next: u16,
}

#[derive(Clone, Copy)]
struct SplitQueue {
    index: u16,
    size: u16,
    descriptor: u64,
    available: u64,
    used: u64,
    notify_address: u64,
    mapping_physical: u64,
    avail_index: u16,
    used_index: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CtrlHeader {
    command_type: u32,
    flags: u32,
    fence_id: u64,
    context_id: u32,
    ring_index: u8,
    padding: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct DisplayOne {
    rect: Rect,
    enabled: u32,
    flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DisplayInfoResponse {
    header: CtrlHeader,
    modes: [DisplayOne; VIRTIO_GPU_MAX_SCANOUTS],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ResourceCreate2d {
    header: CtrlHeader,
    resource_id: u32,
    format: u32,
    width: u32,
    height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ResourceAttachBacking {
    header: CtrlHeader,
    resource_id: u32,
    entry_count: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MemoryEntry {
    address: u64,
    length: u32,
    padding: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct SetScanout {
    header: CtrlHeader,
    rect: Rect,
    scanout_id: u32,
    resource_id: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct TransferToHost2d {
    header: CtrlHeader,
    rect: Rect,
    offset: u64,
    resource_id: u32,
    padding: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ResourceFlush {
    header: CtrlHeader,
    rect: Rect,
    resource_id: u32,
    padding: u32,
}

#[derive(Clone, Copy)]
struct ScanoutBuffer {
    resource_id: u32,
    physical: u64,
    bytes: u64,
}

impl ScanoutBuffer {
    const EMPTY: Self = Self { resource_id: 0, physical: 0, bytes: 0 };
}

struct Transport {
    function: PciFunction,
    device_id: DeviceId,
    driver_id: u64,
    caps: ModernCapabilities,
    control: SplitQueue,
    cursor: SplitQueue,
    workspace_physical: u64,
    framebuffer_physical: u64,
    framebuffer_bytes: u64,
    negotiated_low: u32,
    negotiated_high: u32,
    scanout_id: u32,
    width: u32,
    height: u32,
    buffers: [ScanoutBuffer; PRESENT_BUFFER_COUNT],
    front_buffer: usize,
    frame_sequence: u64,
    next_fence: u64,
    completed_fence: u64,
    damage_uploads: u64,
    presentation_suspended: bool,
}

static LIVE_TRANSPORT: SpinLock<Option<Transport>> = SpinLock::new(None);

#[must_use]
pub fn probe() -> VirtioGpuProbe {
    let Some(function) = find_function() else {
        return VirtioGpuProbe::default();
    };
    describe(function)
}

fn find_function() -> Option<PciFunction> {
    pci::find_first(|entry| {
        entry.vendor_id == VENDOR_VIRTIO && entry.device_id == VIRTIO_GPU_MODERN_DEVICE
    })
}

fn describe(function: PciFunction) -> VirtioGpuProbe {
    let mut resources = [crate::device::Resource::None; 8];
    pci::read_resources(function, &mut resources);
    let memory_bars = resources
        .iter()
        .filter(|resource| matches!(resource, crate::device::Resource::Mmio { .. }))
        .count();
    let command = pci::read_u16(function.bus, function.device, function.function, 0x04);
    VirtioGpuProbe {
        present: true,
        bus: function.bus,
        device: function.device,
        function: function.function,
        device_id: function.device_id,
        memory_bars,
        bus_master_enabled: command & (1 << 2) != 0,
    }
}

pub fn initialize_transport(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
    identity_map_limit: u64,
) -> Result<VirtioGpuTransportReport, &'static str> {
    let function = find_function().ok_or("no modern VirtIO-GPU PCI function found")?;
    let (device_id, driver_id) = forgebus::claim_pci_function(function, b"titan-virtio-gpu", 2)?;
    // Normalize firmware/QEMU state: ownership is established before K13.B
    // permits any DMA, even if a previous firmware path happened to leave the
    // PCI command bit set.
    pci::disable_bus_master(function);
    forgebus::establish_dma_domain(device_id, 64, true)?;

    // MMIO decode is safe after ForgeBus ownership. Bus mastering remains off
    // until all queue and DMA pages are allocated and published.
    pci::enable_memory_decode(function);
    let caps = discover_capabilities(function, allocator, kernel_cr3, identity_map_limit)?;
    let (negotiated_low, negotiated_high) = negotiate_features(caps.common)?;

    let control = setup_queue(allocator, device_id, caps, CONTROL_QUEUE_INDEX)?;
    let cursor = setup_queue(allocator, device_id, caps, CURSOR_QUEUE_INDEX)?;
    let workspace = forgebus::allocate_dma(
        allocator,
        device_id,
        WORKSPACE_BYTES,
        DmaDirection::Bidirectional,
    )?;
    unsafe { ptr::write_bytes(workspace.physical as *mut u8, 0, workspace.bytes as usize) };

    // DMA authorization happens last. A failed capability walk or queue setup
    // therefore cannot leave a bus-mastering device pointed at arbitrary RAM.
    pci::enable_bus_master(function);
    if pci::read_u16(function.bus, function.device, function.function, 0x04) & (1 << 2) == 0 {
        fail_device(function, caps.common);
        return Err("VirtIO-GPU PCI bus mastering did not enable");
    }
    set_status_bits(caps.common, STATUS_DRIVER_OK);
    if read8(caps.common.base + COMMON_STATUS) & STATUS_DRIVER_OK == 0 {
        fail_device(function, caps.common);
        return Err("VirtIO-GPU did not enter DRIVER_OK state");
    }

    let mut transport = Transport {
        function,
        device_id,
        driver_id,
        caps,
        control,
        cursor,
        workspace_physical: workspace.physical,
        framebuffer_physical: 0,
        framebuffer_bytes: 0,
        negotiated_low,
        negotiated_high,
        scanout_id: 0,
        width: 0,
        height: 0,
        buffers: [ScanoutBuffer::EMPTY; PRESENT_BUFFER_COUNT],
        front_buffer: 0,
        frame_sequence: 0,
        next_fence: 1,
        completed_fence: 0,
        damage_uploads: 0,
        presentation_suspended: false,
    };

    let (scanout_id, source_width, source_height) = transport.get_display_info()?;
    let width = core::cmp::min(source_width, MAX_SCANOUT_WIDTH);
    let height = core::cmp::min(source_height, MAX_SCANOUT_HEIGHT);
    if width == 0 || height == 0 {
        transport.fail();
        return Err("VirtIO-GPU reported no usable scanout mode");
    }
    transport.scanout_id = scanout_id;
    transport.width = width;
    transport.height = height;

    let framebuffer_bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or("VirtIO-GPU framebuffer size overflow")?;
    let framebuffer = forgebus::allocate_dma(
        allocator,
        device_id,
        framebuffer_bytes,
        DmaDirection::ToDevice,
    )?;
    transport.framebuffer_physical = framebuffer.physical;
    transport.framebuffer_bytes = framebuffer.bytes;
    transport.buffers[0] = ScanoutBuffer {
        resource_id: RESOURCE_ID,
        physical: framebuffer.physical,
        bytes: framebuffer.bytes,
    };
    render_transport_test_pattern(framebuffer.physical, width, height);

    let full = DamageRect { x: 0, y: 0, width, height };
    let command_result = (|| -> Result<(), &'static str> {
        transport.create_resource(RESOURCE_ID, width, height)?;
        transport.attach_backing(RESOURCE_ID, framebuffer.physical, framebuffer_bytes)?;
        transport.set_scanout(RESOURCE_ID, scanout_id, width, height)?;
        transport.transfer_to_host(RESOURCE_ID, full)?;
        transport.flush(RESOURCE_ID, full)?;
        Ok(())
    })();
    if let Err(error) = command_result {
        transport.fail();
        return Err(error);
    }

    // Polling is deliberate during K13.B bootstrap. Reading ISR acknowledges any
    // coalesced queue/config interrupt without requiring the MSI path yet.
    let _ = read8(caps.isr.base);
    forgebus::mark_device_online(device_id)?;

    let device_scanouts = if caps.device.contains(8, 4) {
        read32(caps.device.base + 8)
    } else {
        0
    };
    let report = VirtioGpuTransportReport {
        device_id,
        driver_id,
        control_queue_size: transport.control.size,
        cursor_queue_size: transport.cursor.size,
        scanout_id,
        width,
        height,
        framebuffer_bytes,
        negotiated_features_low: negotiated_low,
        negotiated_features_high: negotiated_high,
        device_scanouts,
        transport_ready: true,
    };
    *LIVE_TRANSPORT.lock() = Some(transport);
    Ok(report)
}

fn discover_capabilities(
    function: PciFunction,
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
    identity_map_limit: u64,
) -> Result<ModernCapabilities, &'static str> {
    let status = pci::read_u16(function.bus, function.device, function.function, 0x06);
    if status & PCI_STATUS_CAP_LIST == 0 {
        return Err("VirtIO-GPU PCI function has no capability list");
    }
    let mut pointer = pci::read_u8(function.bus, function.device, function.function, PCI_CAPABILITY_LIST) & !3;
    let mut common = MmioRegion::EMPTY;
    let mut notify = MmioRegion::EMPTY;
    let mut isr = MmioRegion::EMPTY;
    let mut device = MmioRegion::EMPTY;
    let mut notify_multiplier = 0u32;
    let mut visited = 0usize;

    while pointer >= 0x40 && visited < 48 {
        visited += 1;
        let cap_id = pci::read_u8(function.bus, function.device, function.function, pointer);
        let next = pci::read_u8(function.bus, function.device, function.function, pointer.wrapping_add(1)) & !3;
        if cap_id == PCI_CAP_ID_VENDOR {
            let cap_len = pci::read_u8(function.bus, function.device, function.function, pointer.wrapping_add(2));
            if cap_len >= 16 {
                let cfg_type = pci::read_u8(function.bus, function.device, function.function, pointer.wrapping_add(3));
                let bar = pci::read_u8(function.bus, function.device, function.function, pointer.wrapping_add(4));
                let offset = pci::read_u32(function.bus, function.device, function.function, pointer.wrapping_add(8)) as u64;
                let length = pci::read_u32(function.bus, function.device, function.function, pointer.wrapping_add(12)) as u64;
                if bar < 6 && length != 0 {
                    if let Some(bar_base) = pci::memory_bar_base(function, bar) {
                        let physical_base = bar_base.checked_add(offset).ok_or("VirtIO capability address overflow")?;
                        let physical_end = physical_base.checked_add(length).ok_or("VirtIO capability length overflow")?;
                        if physical_base == 0 {
                            return Err("VirtIO capability has zero physical address");
                        }
                        let base = if physical_end <= identity_map_limit {
                            physical_base
                        } else {
                            let mapped = paging::map_kernel_mmio(allocator, kernel_cr3, physical_base, length)?;
                            serial::println(format_args!(
                                "[MMIO] VirtIO cap type={} phys={:#018x}..{:#018x} -> virt={:#018x}",
                                cfg_type, physical_base, physical_end, mapped
                            ));
                            mapped
                        };
                        let region = MmioRegion { base, length };
                        match cfg_type {
                            VIRTIO_PCI_CAP_COMMON_CFG => common = region,
                            VIRTIO_PCI_CAP_NOTIFY_CFG => {
                                if cap_len < 20 {
                                    return Err("VirtIO notify capability is too short");
                                }
                                notify = region;
                                notify_multiplier = pci::read_u32(
                                    function.bus,
                                    function.device,
                                    function.function,
                                    pointer.wrapping_add(16),
                                );
                            }
                            VIRTIO_PCI_CAP_ISR_CFG => isr = region,
                            VIRTIO_PCI_CAP_DEVICE_CFG => device = region,
                            _ => {}
                        }
                    }
                }
            }
        }
        if next == 0 || next == pointer {
            break;
        }
        pointer = next;
    }

    if !common.contains(0, 56) {
        return Err("VirtIO common configuration capability missing or short");
    }
    if notify.base == 0 || notify_multiplier == 0 {
        return Err("VirtIO notify capability missing");
    }
    if !isr.contains(0, 1) {
        return Err("VirtIO ISR capability missing");
    }
    if !device.contains(0, 16) {
        return Err("VirtIO GPU device configuration capability missing");
    }
    Ok(ModernCapabilities { common, notify, isr, device, notify_multiplier })
}

fn negotiate_features(common: MmioRegion) -> Result<(u32, u32), &'static str> {
    write8(common.base + COMMON_STATUS, 0);
    let mut spins = 0u32;
    while read8(common.base + COMMON_STATUS) != 0 {
        spins = spins.saturating_add(1);
        if spins > 1_000_000 {
            return Err("VirtIO-GPU reset timed out");
        }
        core::hint::spin_loop();
    }
    write8(common.base + COMMON_STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER);

    write32(common.base + COMMON_DFSELECT, 0);
    let offered_low = read32(common.base + COMMON_DF);
    write32(common.base + COMMON_DFSELECT, 1);
    let offered_high = read32(common.base + COMMON_DF);
    if offered_high & VIRTIO_F_VERSION_1_HIGH_BIT == 0 {
        write8(common.base + COMMON_STATUS, STATUS_FAILED);
        return Err("VirtIO-GPU does not advertise VERSION_1");
    }

    // K13.B deliberately does not request ACCESS_PLATFORM until Titanweave's
    // hardware VT-d/AMD-Vi page-table programming is complete.
    let negotiated_low = 0u32;
    let negotiated_high = VIRTIO_F_VERSION_1_HIGH_BIT;
    let _access_platform_offered = offered_high & VIRTIO_F_ACCESS_PLATFORM_HIGH_BIT != 0;
    let _gpu_features = offered_low;

    write32(common.base + COMMON_GFSELECT, 0);
    write32(common.base + COMMON_GF, negotiated_low);
    write32(common.base + COMMON_GFSELECT, 1);
    write32(common.base + COMMON_GF, negotiated_high);
    set_status_bits(common, STATUS_FEATURES_OK);
    if read8(common.base + COMMON_STATUS) & STATUS_FEATURES_OK == 0 {
        write8(common.base + COMMON_STATUS, STATUS_FAILED);
        return Err("VirtIO-GPU rejected negotiated feature set");
    }
    Ok((negotiated_low, negotiated_high))
}

fn setup_queue(
    allocator: &mut FrameAllocator<'_>,
    device_id: DeviceId,
    caps: ModernCapabilities,
    index: u16,
) -> Result<SplitQueue, &'static str> {
    let common = caps.common.base;
    write16(common + COMMON_Q_SELECT, index);
    let offered = read16(common + COMMON_Q_SIZE);
    if offered < QUEUE_MINIMUM || !offered.is_power_of_two() {
        return Err("VirtIO-GPU offered invalid virtqueue size");
    }
    if read16(common + COMMON_Q_ENABLE) != 0 {
        return Err("VirtIO-GPU virtqueue is already enabled");
    }
    let size = core::cmp::min(offered, MAX_QUEUE_SIZE);
    let descriptor_bytes = usize::from(size) * size_of::<Descriptor>();
    let available_offset = descriptor_bytes;
    let available_bytes = 6usize + usize::from(size) * 2;
    let used_offset = align_up(available_offset + available_bytes, 4);
    let used_bytes = 6usize + usize::from(size) * 8;
    let queue_bytes = used_offset.checked_add(used_bytes).ok_or("VirtIO queue size overflow")?;
    let mapping = forgebus::allocate_dma(
        allocator,
        device_id,
        queue_bytes as u64,
        DmaDirection::Bidirectional,
    )?;
    unsafe { ptr::write_bytes(mapping.physical as *mut u8, 0, mapping.bytes as usize) };

    let descriptor = mapping.physical;
    let available = descriptor + available_offset as u64;
    let used = descriptor + used_offset as u64;
    write16(common + COMMON_Q_SIZE, size);
    write16(common + COMMON_Q_MSIX, 0xffff);
    write32(common + COMMON_Q_DESCLO, descriptor as u32);
    write32(common + COMMON_Q_DESCHI, (descriptor >> 32) as u32);
    write32(common + COMMON_Q_AVAILLO, available as u32);
    write32(common + COMMON_Q_AVAILHI, (available >> 32) as u32);
    write32(common + COMMON_Q_USEDLO, used as u32);
    write32(common + COMMON_Q_USEDHI, (used >> 32) as u32);
    let notify_offset = u64::from(read16(common + COMMON_Q_NOFF))
        .checked_mul(u64::from(caps.notify_multiplier))
        .ok_or("VirtIO notify offset overflow")?;
    if !caps.notify.contains(notify_offset, 2) {
        return Err("VirtIO queue notify address lies outside notify capability");
    }
    write16(common + COMMON_Q_ENABLE, 1);
    if read16(common + COMMON_Q_ENABLE) != 1 {
        return Err("VirtIO-GPU queue did not enable");
    }
    Ok(SplitQueue {
        index,
        size,
        descriptor,
        available,
        used,
        notify_address: caps.notify.base + notify_offset,
        mapping_physical: mapping.physical,
        avail_index: 0,
        used_index: 0,
    })
}

impl Transport {
    fn submit(&mut self, command_bytes: usize, response_bytes: usize) -> Result<u32, &'static str> {
        if command_bytes == 0 || command_bytes > WORKSPACE_RESPONSE_OFFSET as usize {
            return Err("VirtIO-GPU command exceeds workspace");
        }
        let response_capacity = WORKSPACE_BYTES
            .checked_sub(WORKSPACE_RESPONSE_OFFSET)
            .ok_or("VirtIO-GPU response workspace underflow")? as usize;
        if response_bytes < size_of::<CtrlHeader>() || response_bytes > response_capacity {
            return Err("VirtIO-GPU response exceeds workspace");
        }
        let response = self.workspace_physical + WORKSPACE_RESPONSE_OFFSET;
        unsafe { ptr::write_bytes(response as *mut u8, 0, response_bytes) };

        let descriptors = self.control.descriptor as *mut Descriptor;
        unsafe {
            ptr::write_volatile(
                descriptors.add(0),
                Descriptor {
                    address: self.workspace_physical,
                    length: command_bytes as u32,
                    flags: DESC_NEXT,
                    next: 1,
                },
            );
            ptr::write_volatile(
                descriptors.add(1),
                Descriptor {
                    address: response,
                    length: response_bytes as u32,
                    flags: DESC_WRITE,
                    next: 0,
                },
            );
        }

        let ring_slot = self.control.avail_index % self.control.size;
        unsafe {
            ptr::write_volatile((self.control.available + 4 + u64::from(ring_slot) * 2) as *mut u16, 0);
        }
        compiler_fence(Ordering::Release);
        self.control.avail_index = self.control.avail_index.wrapping_add(1);
        unsafe { ptr::write_volatile((self.control.available + 2) as *mut u16, self.control.avail_index) };
        compiler_fence(Ordering::SeqCst);
        write16(self.control.notify_address, self.control.index);

        let target = self.control.used_index.wrapping_add(1);
        let mut spins = 0u64;
        loop {
            let used = unsafe { ptr::read_volatile((self.control.used + 2) as *const u16) };
            if used == target {
                self.control.used_index = used;
                break;
            }
            spins = spins.saturating_add(1);
            if spins > COMMAND_TIMEOUT_SPINS {
                self.fail();
                return Err("VirtIO-GPU control command timed out");
            }
            core::hint::spin_loop();
        }
        compiler_fence(Ordering::Acquire);
        Ok(unsafe { ptr::read_volatile(response as *const u32) })
    }

    fn get_display_info(&mut self) -> Result<(u32, u32, u32), &'static str> {
        self.clear_workspace();
        unsafe {
            ptr::write(
                self.workspace_physical as *mut CtrlHeader,
                CtrlHeader { command_type: VIRTIO_GPU_CMD_GET_DISPLAY_INFO, ..CtrlHeader::default() },
            );
        }
        let response_type = self.submit(size_of::<CtrlHeader>(), size_of::<DisplayInfoResponse>())?;
        if response_type != VIRTIO_GPU_RESP_OK_DISPLAY_INFO {
            return Err("VirtIO-GPU GET_DISPLAY_INFO failed");
        }
        let response = unsafe {
            ptr::read_volatile((self.workspace_physical + WORKSPACE_RESPONSE_OFFSET) as *const DisplayInfoResponse)
        };
        for (index, mode) in response.modes.iter().enumerate() {
            if mode.enabled != 0 && mode.rect.width != 0 && mode.rect.height != 0 {
                return Ok((index as u32, mode.rect.width, mode.rect.height));
            }
        }
        // A secondary QEMU GPU can exist before its console becomes the active
        // frontend. Prefer any advertised non-zero rectangle even when the
        // enabled bit is not yet set, then use a conservative bootstrap mode
        // when the device reports at least one scanout but no dimensions.
        for (index, mode) in response.modes.iter().enumerate() {
            if mode.rect.width != 0 && mode.rect.height != 0 {
                return Ok((index as u32, mode.rect.width, mode.rect.height));
            }
        }
        if self.caps.device.contains(8, 4) && read32(self.caps.device.base + 8) != 0 {
            return Ok((0, 1024, 768));
        }
        Err("VirtIO-GPU reports no scanout capability")
    }

    fn create_resource(&mut self, resource_id: u32, width: u32, height: u32) -> Result<(), &'static str> {
        self.clear_workspace();
        let command = ResourceCreate2d {
            header: CtrlHeader { command_type: VIRTIO_GPU_CMD_RESOURCE_CREATE_2D, ..CtrlHeader::default() },
            resource_id,
            format: VIRTIO_GPU_FORMAT_B8G8R8X8_UNORM,
            width,
            height,
        };
        unsafe { ptr::write(self.workspace_physical as *mut ResourceCreate2d, command) };
        self.expect_nodata(size_of::<ResourceCreate2d>(), "RESOURCE_CREATE_2D")
    }

    fn attach_backing(&mut self, resource_id: u32, physical: u64, bytes: u64) -> Result<(), &'static str> {
        let length = u32::try_from(bytes).map_err(|_| "VirtIO-GPU backing exceeds 32-bit entry length")?;
        self.clear_workspace();
        let command = ResourceAttachBacking {
            header: CtrlHeader { command_type: VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING, ..CtrlHeader::default() },
            resource_id,
            entry_count: 1,
        };
        unsafe {
            ptr::write(self.workspace_physical as *mut ResourceAttachBacking, command);
            ptr::write(
                (self.workspace_physical + size_of::<ResourceAttachBacking>() as u64) as *mut MemoryEntry,
                MemoryEntry { address: physical, length, padding: 0 },
            );
        }
        self.expect_nodata(size_of::<ResourceAttachBacking>() + size_of::<MemoryEntry>(), "RESOURCE_ATTACH_BACKING")
    }

    fn set_scanout(&mut self, resource_id: u32, scanout_id: u32, width: u32, height: u32) -> Result<(), &'static str> {
        self.clear_workspace();
        let command = SetScanout {
            header: CtrlHeader { command_type: VIRTIO_GPU_CMD_SET_SCANOUT, ..CtrlHeader::default() },
            rect: Rect { x: 0, y: 0, width, height },
            scanout_id,
            resource_id,
        };
        unsafe { ptr::write(self.workspace_physical as *mut SetScanout, command) };
        self.expect_nodata(size_of::<SetScanout>(), "SET_SCANOUT")
    }

    fn transfer_to_host(&mut self, resource_id: u32, damage: DamageRect) -> Result<(), &'static str> {
        let damage = damage.clipped(self.width, self.height)?;
        let offset = damage.byte_offset(self.width).ok_or("VirtIO-GPU damage offset overflow")?;
        self.clear_workspace();
        let command = TransferToHost2d {
            header: CtrlHeader { command_type: VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D, ..CtrlHeader::default() },
            rect: Rect { x: damage.x, y: damage.y, width: damage.width, height: damage.height },
            offset,
            resource_id,
            padding: 0,
        };
        unsafe { ptr::write(self.workspace_physical as *mut TransferToHost2d, command) };
        self.expect_nodata(size_of::<TransferToHost2d>(), "TRANSFER_TO_HOST_2D")
    }

    fn flush(&mut self, resource_id: u32, damage: DamageRect) -> Result<(), &'static str> {
        let damage = damage.clipped(self.width, self.height)?;
        self.clear_workspace();
        let command = ResourceFlush {
            header: CtrlHeader { command_type: VIRTIO_GPU_CMD_RESOURCE_FLUSH, ..CtrlHeader::default() },
            rect: Rect { x: damage.x, y: damage.y, width: damage.width, height: damage.height },
            resource_id,
            padding: 0,
        };
        unsafe { ptr::write(self.workspace_physical as *mut ResourceFlush, command) };
        self.expect_nodata(size_of::<ResourceFlush>(), "RESOURCE_FLUSH")
    }

    fn expect_nodata(&mut self, command_bytes: usize, _name: &'static str) -> Result<(), &'static str> {
        let response = self.submit(command_bytes, size_of::<CtrlHeader>())?;
        if response != VIRTIO_GPU_RESP_OK_NODATA {
            return Err("VirtIO-GPU command returned an error response");
        }
        Ok(())
    }

    fn flush_fenced(&mut self, resource_id: u32, damage: DamageRect, fence_id: u64) -> Result<(), &'static str> {
        let damage = damage.clipped(self.width, self.height)?;
        self.clear_workspace();
        let command = ResourceFlush {
            header: CtrlHeader {
                command_type: VIRTIO_GPU_CMD_RESOURCE_FLUSH,
                flags: VIRTIO_GPU_FLAG_FENCE,
                fence_id,
                ..CtrlHeader::default()
            },
            rect: Rect { x: damage.x, y: damage.y, width: damage.width, height: damage.height },
            resource_id,
            padding: 0,
        };
        unsafe { ptr::write(self.workspace_physical as *mut ResourceFlush, command) };
        let response_type = self.submit(size_of::<ResourceFlush>(), size_of::<CtrlHeader>())?;
        if response_type != VIRTIO_GPU_RESP_OK_NODATA {
            return Err("VirtIO-GPU fenced flush returned an error response");
        }
        let response = unsafe {
            ptr::read_volatile((self.workspace_physical + WORKSPACE_RESPONSE_OFFSET) as *const CtrlHeader)
        };
        if response.flags & VIRTIO_GPU_FLAG_FENCE == 0 || response.fence_id != fence_id {
            return Err("VirtIO-GPU fence completion did not match submitted fence");
        }
        self.completed_fence = fence_id;
        Ok(())
    }

    fn present_damage(&mut self, damage: DamageRect, pattern_seed: u64) -> Result<VirtioGpuPresentResult, &'static str> {
        let damage = damage.clipped(self.width, self.height)?;
        if self.completed_fence + 1 < self.next_fence {
            return Err("VirtIO-GPU presentation queue exceeded in-flight fence bound");
        }
        let next_index = (self.front_buffer + 1) % PRESENT_BUFFER_COUNT;
        let buffer = self.buffers[next_index];
        if buffer.resource_id == 0 || buffer.physical == 0 {
            return Err("VirtIO-GPU presentation buffer is not initialized");
        }

        render_compositor_damage(buffer.physical, self.width, self.height, damage, pattern_seed);
        self.transfer_to_host(buffer.resource_id, damage)?;
        self.set_scanout(buffer.resource_id, self.scanout_id, self.width, self.height)?;
        let fence_id = self.next_fence;
        self.next_fence = self.next_fence.checked_add(1).ok_or("VirtIO-GPU fence id overflow")?;
        self.flush_fenced(buffer.resource_id, damage, fence_id)?;
        self.front_buffer = next_index;
        self.frame_sequence = self.frame_sequence.checked_add(1).ok_or("VirtIO-GPU frame sequence overflow")?;
        self.damage_uploads = self.damage_uploads.saturating_add(1);
        Ok(VirtioGpuPresentResult {
            frame_sequence: self.frame_sequence,
            fence_id,
            resource_id: buffer.resource_id,
            damage,
        })
    }

    fn clear_workspace(&self) {
        unsafe { ptr::write_bytes(self.workspace_physical as *mut u8, 0, WORKSPACE_RESPONSE_OFFSET as usize) };
    }

    fn fail(&self) {
        fail_device(self.function, self.caps.common);
    }
}

pub fn initialize_presentation(
    allocator: &mut FrameAllocator<'_>,
) -> Result<VirtioGpuPresentationReport, &'static str> {
    let mut guard = LIVE_TRANSPORT.lock();
    let transport = guard.as_mut().ok_or("VirtIO-GPU live transport is unavailable")?;
    if transport.width == 0 || transport.height == 0 || transport.buffers[0].resource_id == 0 {
        return Err("VirtIO-GPU transport has no qualified scanout buffer");
    }

    let requested_bytes = u64::from(transport.width)
        .checked_mul(u64::from(transport.height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or("VirtIO-GPU presentation buffer size overflow")?;
    let full = DamageRect { x: 0, y: 0, width: transport.width, height: transport.height };

    for index in 1..PRESENT_BUFFER_COUNT {
        if transport.buffers[index].resource_id != 0 {
            continue;
        }
        let mapping = forgebus::allocate_dma(
            allocator,
            transport.device_id,
            requested_bytes,
            DmaDirection::ToDevice,
        )?;
        let resource_id = RESOURCE_ID + index as u32;
        transport.buffers[index] = ScanoutBuffer {
            resource_id,
            physical: mapping.physical,
            bytes: mapping.bytes,
        };
        render_transport_test_pattern(mapping.physical, transport.width, transport.height);
        transport.create_resource(resource_id, transport.width, transport.height)?;
        transport.attach_backing(resource_id, mapping.physical, requested_bytes)?;
        transport.transfer_to_host(resource_id, full)?;
        transport.flush(resource_id, full)?;
    }

    let damage_a = DamageRect {
        x: core::cmp::min(48, transport.width.saturating_sub(1)),
        y: core::cmp::min(72, transport.height.saturating_sub(1)),
        width: core::cmp::min(320, transport.width),
        height: core::cmp::min(180, transport.height),
    };
    let damage_b = DamageRect {
        x: transport.width / 3,
        y: transport.height / 3,
        width: core::cmp::max(1, transport.width / 4),
        height: core::cmp::max(1, transport.height / 5),
    };
    let damage_c = DamageRect {
        x: transport.width.saturating_sub(core::cmp::min(280, transport.width)),
        y: transport.height.saturating_sub(core::cmp::min(140, transport.height)),
        width: core::cmp::min(280, transport.width),
        height: core::cmp::min(140, transport.height),
    };

    let _ = transport.present_damage(damage_a, 1)?;
    let _ = transport.present_damage(damage_b, 2)?;
    let last = transport.present_damage(damage_c, 3)?;

    Ok(VirtioGpuPresentationReport {
        buffers: PRESENT_BUFFER_COUNT as u32,
        frames_presented: transport.frame_sequence,
        damage_uploads: transport.damage_uploads,
        last_fence: last.fence_id,
        front_resource: last.resource_id,
        fallback_armed: true,
    })
}

pub fn present_compositor_frame(pattern_seed: u64) -> Result<VirtioGpuPresentResult, &'static str> {
    let mut guard = LIVE_TRANSPORT.lock();
    let transport = guard.as_mut().ok_or("VirtIO-GPU live transport is unavailable")?;
    if transport.presentation_suspended {
        return Err("VirtIO-GPU presentation is suspended for recovery");
    }
    let width = core::cmp::max(1, transport.width / 5);
    let height = core::cmp::max(1, transport.height / 6);
    let x_span = transport.width.saturating_sub(width).saturating_add(1);
    let y_span = transport.height.saturating_sub(height).saturating_add(1);
    let x = if x_span == 0 { 0 } else { ((pattern_seed.saturating_mul(73)) % u64::from(x_span)) as u32 };
    let y = if y_span == 0 { 0 } else { ((pattern_seed.saturating_mul(41)) % u64::from(y_span)) as u32 };
    transport.present_damage(DamageRect { x, y, width, height }, pattern_seed)
}

#[must_use]
pub fn presentation_ready() -> bool {
    LIVE_TRANSPORT
        .lock()
        .as_ref()
        .is_some_and(|transport| {
            !transport.presentation_suspended
                && transport.buffers.iter().all(|buffer| buffer.resource_id != 0)
        })
}

/// K13.D controlled recovery hook. This fences new presentation work without
/// discarding the already-qualified PCI ownership, DMA domain or queue state.
/// The firmware GOP path remains available while presentation is suspended.
pub fn suspend_presentation_for_recovery() -> Result<VirtioGpuRecoveryReport, &'static str> {
    let mut guard = LIVE_TRANSPORT.lock();
    let transport = guard.as_mut().ok_or("VirtIO-GPU live transport is unavailable")?;
    transport.presentation_suspended = true;
    let command = pci::read_u16(
        transport.function.bus,
        transport.function.device,
        transport.function.function,
        0x04,
    );
    let status = read8(transport.caps.common.base + COMMON_STATUS);
    Ok(VirtioGpuRecoveryReport {
        device_id: transport.device_id,
        frame_sequence: transport.frame_sequence,
        completed_fence: transport.completed_fence,
        bus_master_enabled: command & (1 << 2) != 0,
        driver_ok: status & STATUS_DRIVER_OK != 0,
    })
}

/// Rearm presentation after K13.D verifies that the owned transport is still
/// DRIVER_OK, bus mastering is still bounded by ForgeBus, and all scanout
/// buffers remain valid. Full PCI FLR/slot reset is intentionally not claimed
/// by this checkpoint.
pub fn resume_presentation_after_recovery() -> Result<VirtioGpuRecoveryReport, &'static str> {
    let mut guard = LIVE_TRANSPORT.lock();
    let transport = guard.as_mut().ok_or("VirtIO-GPU live transport is unavailable")?;
    let command = pci::read_u16(
        transport.function.bus,
        transport.function.device,
        transport.function.function,
        0x04,
    );
    let status = read8(transport.caps.common.base + COMMON_STATUS);
    if command & (1 << 2) == 0 {
        return Err("VirtIO-GPU recovery found bus mastering disabled");
    }
    if status & STATUS_DRIVER_OK == 0 {
        return Err("VirtIO-GPU recovery found device outside DRIVER_OK");
    }
    if !transport.buffers.iter().all(|buffer| buffer.resource_id != 0 && buffer.physical != 0) {
        return Err("VirtIO-GPU recovery found an invalid presentation buffer");
    }
    transport.presentation_suspended = false;
    Ok(VirtioGpuRecoveryReport {
        device_id: transport.device_id,
        frame_sequence: transport.frame_sequence,
        completed_fence: transport.completed_fence,
        bus_master_enabled: true,
        driver_ok: true,
    })
}

/// Fence the accelerated backend after a live presentation failure. The K12 GOP
/// framebuffer remains mapped and visible as the recovery scanout; K13.C never
/// escalates privileges or retries DMA with broader access after a stall/fault.
pub fn disable_accelerated_presentation() {
    let mut guard = LIVE_TRANSPORT.lock();
    if let Some(transport) = guard.as_ref() {
        transport.fail();
    }
    *guard = None;
}

fn render_compositor_damage(
    physical: u64,
    display_width: u32,
    display_height: u32,
    damage: DamageRect,
    seed: u64,
) {
    if physical == 0 || display_width == 0 || display_height == 0 {
        return;
    }
    let Ok(damage) = damage.clipped(display_width, display_height) else { return };
    let stride = display_width as usize;
    let accent = match seed % 4 {
        0 => 0x0088_45ffu32,
        1 => 0x006b_38d1u32,
        2 => 0x003d_8bffu32,
        _ => 0x00a0_58ffu32,
    };
    for y in damage.y..damage.y.saturating_add(damage.height) {
        for x in damage.x..damage.x.saturating_add(damage.width) {
            let border = x == damage.x
                || y == damage.y
                || x + 1 == damage.x.saturating_add(damage.width)
                || y + 1 == damage.y.saturating_add(damage.height);
            let pixel = if border { accent } else if ((x / 16) + (y / 16) + seed as u32) & 1 == 0 {
                0x002a_3048u32
            } else {
                0x0018_1d2du32
            };
            unsafe {
                ptr::write_volatile((physical as *mut u32).add(y as usize * stride + x as usize), pixel)
            };
        }
    }
}

fn render_transport_test_pattern(physical: u64, width: u32, height: u32) {
    let stride = width as usize;
    for y in 0..height as usize {
        for x in 0..width as usize {
            let top = y < 56;
            let bottom = y + 44 >= height as usize;
            let rail = x < core::cmp::min(220, width as usize / 4);
            let pixel = if top || bottom {
                0x0015_1826u32
            } else if rail {
                0x001d_2435u32
            } else if ((x / 96) + (y / 72)) & 1 == 0 {
                0x0023_2940u32
            } else {
                0x001b_2032u32
            };
            let accent = if y >= 56 && y < 62 { 0x0088_45ffu32 } else { pixel };
            unsafe { ptr::write_volatile((physical as *mut u32).add(y * stride + x), accent) };
        }
    }
}

fn fail_device(function: PciFunction, common: MmioRegion) {
    if common.base != 0 {
        write8(common.base + COMMON_STATUS, STATUS_FAILED);
        write8(common.base + COMMON_STATUS, 0);
    }
    pci::disable_bus_master(function);
}

fn set_status_bits(common: MmioRegion, bits: u8) {
    let value = read8(common.base + COMMON_STATUS);
    write8(common.base + COMMON_STATUS, value | bits);
}

fn read8(address: u64) -> u8 {
    unsafe { ptr::read_volatile(address as *const u8) }
}
fn read16(address: u64) -> u16 {
    unsafe { ptr::read_volatile(address as *const u16) }
}
fn read32(address: u64) -> u32 {
    unsafe { ptr::read_volatile(address as *const u32) }
}
fn write8(address: u64, value: u8) {
    unsafe { ptr::write_volatile(address as *mut u8, value) }
}
fn write16(address: u64, value: u16) {
    unsafe { ptr::write_volatile(address as *mut u16, value) }
}
fn write32(address: u64, value: u32) {
    unsafe { ptr::write_volatile(address as *mut u32, value) }
}

const fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}
