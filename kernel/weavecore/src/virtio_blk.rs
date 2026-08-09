use crate::arch::x86_64::port::{inb, inl, inw, outb, outl, outw};
use crate::memory::{FrameAllocator, FRAME_SIZE};
use crate::pci::{self, PciFunction};
use core::ptr;
use core::sync::atomic::{compiler_fence, Ordering};

const VIRTIO_VENDOR_ID: u16 = 0x1af4;
const VIRTIO_BLK_TRANSITIONAL: u16 = 0x1001;
const VIRTIO_BLK_MODERN: u16 = 0x1042;

const REG_DEVICE_FEATURES: u16 = 0x00;
const REG_GUEST_FEATURES: u16 = 0x04;
const REG_QUEUE_PFN: u16 = 0x08;
const REG_QUEUE_SIZE: u16 = 0x0c;
const REG_QUEUE_SELECT: u16 = 0x0e;
const REG_QUEUE_NOTIFY: u16 = 0x10;
const REG_DEVICE_STATUS: u16 = 0x12;
const REG_ISR_STATUS: u16 = 0x13;

const STATUS_ACKNOWLEDGE: u8 = 1;
const STATUS_DRIVER: u8 = 2;
const STATUS_DRIVER_OK: u8 = 4;
const STATUS_FEATURES_OK: u8 = 8;
const STATUS_FAILED: u8 = 128;

const DESC_NEXT: u16 = 1;
const DESC_WRITE: u16 = 2;
const REQUEST_IN: u32 = 0;
const QUEUE_INDEX: u16 = 0;
const MAX_QUEUE_ENTRIES: usize = 1024;
const REQUEST_PAGES: u64 = 1;

#[derive(Clone, Copy, Debug)]
pub struct VirtioBlockProbe {
    pub function: PciFunction,
    pub modern: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct VirtioInitializationReport {
    pub probe: VirtioBlockProbe,
    pub queue_size: u16,
    pub boot_signature: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Descriptor {
    address: u64,
    length: u32,
    flags: u16,
    next: u16,
}

#[repr(C)]
struct RequestHeader {
    request_type: u32,
    reserved: u32,
    sector: u64,
}

pub fn probe() -> Option<VirtioBlockProbe> {
    pci::find_first(|function| {
        function.vendor_id == VIRTIO_VENDOR_ID
            && matches!(function.device_id, VIRTIO_BLK_TRANSITIONAL | VIRTIO_BLK_MODERN)
    })
    .map(|function| VirtioBlockProbe {
        modern: function.device_id == VIRTIO_BLK_MODERN,
        function,
    })
}

/// Initializes a transitional VirtIO-blk transport and performs a reusable queue and verifies it with an initial DMA read.
///
/// K6 deliberately asks QEMU for the legacy/transitional interface. The modern
/// PCI capability transport is a later DeviceKit storage-driver milestone.
pub fn initialize_and_verify(
    allocator: &mut FrameAllocator<'_>,
) -> Result<VirtioInitializationReport, &'static str> {
    let probe = probe().ok_or("no VirtIO block PCI function found")?;
    if probe.modern {
        return Err("K6 queue initialization requires the transitional VirtIO interface");
    }

    let bar0 = pci::read_u32(
        probe.function.bus,
        probe.function.device,
        probe.function.function,
        0x10,
    );
    if bar0 & 1 == 0 {
        return Err("VirtIO transitional BAR0 is not an I/O BAR");
    }
    let io_base_u32 = bar0 & !3;
    let io_base = u16::try_from(io_base_u32).map_err(|_| "VirtIO I/O BAR exceeds 16-bit port space")?;
    pci::enable_io_and_bus_master(probe.function);

    unsafe {
        outb(io_base + REG_DEVICE_STATUS, 0);
        outb(io_base + REG_DEVICE_STATUS, STATUS_ACKNOWLEDGE);
        outb(
            io_base + REG_DEVICE_STATUS,
            STATUS_ACKNOWLEDGE | STATUS_DRIVER,
        );
        let _features = inl(io_base + REG_DEVICE_FEATURES);
        outl(io_base + REG_GUEST_FEATURES, 0);
        outb(
            io_base + REG_DEVICE_STATUS,
            STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK,
        );
        if inb(io_base + REG_DEVICE_STATUS) & STATUS_FEATURES_OK == 0 {
            outb(io_base + REG_DEVICE_STATUS, STATUS_FAILED);
            return Err("VirtIO device rejected the negotiated feature set");
        }
        outw(io_base + REG_QUEUE_SELECT, QUEUE_INDEX);
    }
    let existing_queue = unsafe { inl(io_base + REG_QUEUE_PFN) };
    if existing_queue != 0 {
        unsafe { outb(io_base + REG_DEVICE_STATUS, STATUS_FAILED) };
        return Err("VirtIO block request queue is already active");
    }
    let offered = unsafe { inw(io_base + REG_QUEUE_SIZE) };
    if offered < 3 {
        unsafe { outb(io_base + REG_DEVICE_STATUS, STATUS_FAILED) };
        return Err("VirtIO block request queue is too small");
    }
    let queue_size = offered as usize;
    if queue_size > MAX_QUEUE_ENTRIES {
        unsafe { outb(io_base + REG_DEVICE_STATUS, STATUS_FAILED) };
        return Err("VirtIO block request queue exceeds K6 safety bound");
    }
    let descriptor_bytes = queue_size * core::mem::size_of::<Descriptor>();
    let available_offset = descriptor_bytes;
    let available_bytes = 6 + queue_size * 2;
    let used_offset = align_up(available_offset + available_bytes, FRAME_SIZE as usize);
    let used_bytes = 6 + queue_size * 8;
    let queue_bytes = used_offset + used_bytes;
    let queue_pages = u64::try_from(align_up(queue_bytes, FRAME_SIZE as usize) / FRAME_SIZE as usize)
        .map_err(|_| "VirtIO queue page count overflow")?;

    let queue_physical = allocator
        .allocate_contiguous(queue_pages)
        .ok_or("no contiguous memory for VirtIO queue")?;
    unsafe { ptr::write_bytes(queue_physical as *mut u8, 0, (queue_pages * FRAME_SIZE) as usize) };
    let request_physical = allocator
        .allocate_contiguous(REQUEST_PAGES)
        .ok_or("no memory for VirtIO block request")?;
    unsafe { ptr::write_bytes(request_physical as *mut u8, 0, FRAME_SIZE as usize) };

    let header_address = request_physical;
    let data_address = request_physical + 512;
    let status_address = request_physical + 1024;
    unsafe {
        ptr::write(
            header_address as *mut RequestHeader,
            RequestHeader {
                request_type: REQUEST_IN,
                reserved: 0,
                sector: 0,
            },
        );
        ptr::write(status_address as *mut u8, 0xff);
        let descriptors = queue_physical as *mut Descriptor;
        ptr::write(
            descriptors.add(0),
            Descriptor {
                address: header_address,
                length: core::mem::size_of::<RequestHeader>() as u32,
                flags: DESC_NEXT,
                next: 1,
            },
        );
        ptr::write(
            descriptors.add(1),
            Descriptor {
                address: data_address,
                length: 512,
                flags: DESC_NEXT | DESC_WRITE,
                next: 2,
            },
        );
        ptr::write(
            descriptors.add(2),
            Descriptor {
                address: status_address,
                length: 1,
                flags: DESC_WRITE,
                next: 0,
            },
        );
    }

    let queue_pfn = u32::try_from(queue_physical >> 12)
        .map_err(|_| "VirtIO queue lies above legacy PFN range")?;
    unsafe {
        outl(io_base + REG_QUEUE_PFN, queue_pfn);
        outb(
            io_base + REG_DEVICE_STATUS,
            STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK | STATUS_DRIVER_OK,
        );
        if inb(io_base + REG_DEVICE_STATUS) & STATUS_DRIVER_OK == 0 {
            outb(io_base + REG_DEVICE_STATUS, STATUS_FAILED);
            return Err("VirtIO device did not enter DRIVER_OK state");
        }
    }

    let available = queue_physical + available_offset as u64;
    let used = queue_physical + used_offset as u64;
    unsafe {
        ptr::write_volatile((available + 0) as *mut u16, 0);
        ptr::write_volatile((available + 4) as *mut u16, 0);
        compiler_fence(Ordering::Release);
        ptr::write_volatile((available + 2) as *mut u16, 1);
        outw(io_base + REG_QUEUE_NOTIFY, QUEUE_INDEX);
    }

    let mut spins = 0u64;
    loop {
        let used_index = unsafe { ptr::read_volatile((used + 2) as *const u16) };
        if used_index == 1 {
            break;
        }
        spins += 1;
        if spins > 100_000_000 {
            unsafe { outb(io_base + REG_DEVICE_STATUS, STATUS_FAILED) };
            return Err("VirtIO block request timed out");
        }
        core::hint::spin_loop();
    }
    compiler_fence(Ordering::Acquire);
    let request_status = unsafe { ptr::read_volatile(status_address as *const u8) };
    let _isr = unsafe { inb(io_base + REG_ISR_STATUS) };
    if request_status != 0 {
        return Err("VirtIO block device rejected sector-zero read");
    }
    let signature = unsafe { ptr::read_unaligned((data_address + 510) as *const u16) };
    if signature != 0xaa55 {
        return Err("VirtIO sector zero does not contain a boot signature");
    }

    Ok(VirtioInitializationReport {
        probe,
        queue_size: queue_size as u16,
        boot_signature: signature,
    })
}

const fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}
