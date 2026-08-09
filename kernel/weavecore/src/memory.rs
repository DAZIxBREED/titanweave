use core::marker::PhantomData;
use core::mem::size_of;
use core::ptr;
use titanweave_boot_protocol::{
    uefi_memory_type, BootInfo, MemoryMapInfo, UefiMemoryDescriptorPrefix, UEFI_PAGE_SIZE,
};

pub const FRAME_SIZE: u64 = UEFI_PAGE_SIZE;
/// General-purpose kernel allocations must never consume legacy low memory.
/// The AP trampoline at 0x8000 is reserved explicitly by TitanBoot; everything
/// below 1 MiB remains outside the reclaiming allocator for BIOS data, real-mode
/// bootstrap state, firmware quirks, and future compatibility.
pub const MIN_GENERAL_ALLOC_ADDRESS: u64 = 0x0010_0000;
const MAX_FREE_EXTENTS: usize = 256;

#[derive(Clone, Copy)]
struct Extent {
    start: u64,
    pages: u64,
}

impl Extent {
    const EMPTY: Self = Self { start: 0, pages: 0 };

    fn end(self) -> Option<u64> {
        self.start.checked_add(self.pages.checked_mul(FRAME_SIZE)?)
    }
}

/// Reclaiming physical-frame allocator.
///
/// K2 originally used a one-way cursor. This allocator copies all conventional
/// UEFI ranges into an owned extent table, supports contiguous allocation,
/// deallocation, splitting, and adjacent-range coalescing. It deliberately
/// avoids heap allocation so it is usable before the kernel allocator exists.
pub struct FrameAllocator<'a> {
    free: [Extent; MAX_FREE_EXTENTS],
    free_count: usize,
    total_pages: u64,
    free_pages: u64,
    _boot_lifetime: PhantomData<&'a BootInfo>,
}

impl<'a> FrameAllocator<'a> {
    pub fn new(boot_info: &'a BootInfo) -> Self {
        let mut allocator = Self {
            free: [Extent::EMPTY; MAX_FREE_EXTENTS],
            free_count: 0,
            total_pages: 0,
            free_pages: 0,
            _boot_lifetime: PhantomData,
        };

        for index in 0..boot_info.memory_map.descriptor_count as usize {
            let Some(descriptor) = read_descriptor(&boot_info.memory_map, index) else { break };
            if descriptor.memory_type != uefi_memory_type::CONVENTIONAL || descriptor.page_count == 0 {
                continue;
            }
            // Ignore malformed or unaligned firmware ranges rather than adding
            // ambiguous physical memory to the allocator.
            if descriptor.physical_start % FRAME_SIZE != 0
                || descriptor.page_count.checked_mul(FRAME_SIZE)
                    .and_then(|bytes| descriptor.physical_start.checked_add(bytes)).is_none()
            {
                continue;
            }
            let descriptor_bytes = descriptor.page_count * FRAME_SIZE;
            let descriptor_end = descriptor.physical_start + descriptor_bytes;
            if descriptor_end <= MIN_GENERAL_ALLOC_ADDRESS {
                continue;
            }

            let start = core::cmp::max(descriptor.physical_start, MIN_GENERAL_ALLOC_ADDRESS);
            let trimmed_bytes = descriptor_end - start;
            let pages = trimmed_bytes / FRAME_SIZE;
            if pages == 0 {
                continue;
            }

            if allocator.insert_extent(Extent { start, pages }).is_ok() {
                allocator.total_pages = allocator.total_pages.saturating_add(pages);
                allocator.free_pages = allocator.free_pages.saturating_add(pages);
            }
        }
        allocator.coalesce_all();
        allocator
    }

    pub fn allocate_frame(&mut self) -> Option<u64> {
        self.allocate_contiguous(1)
    }

    pub fn allocate_contiguous(&mut self, pages: u64) -> Option<u64> {
        if pages == 0 { return None; }
        // Best fit reduces fragmentation without requiring dynamic metadata.
        let mut best = None;
        for index in 0..self.free_count {
            let extent = self.free[index];
            if extent.pages >= pages
                && best.map(|best_index: usize| extent.pages < self.free[best_index].pages).unwrap_or(true)
            {
                best = Some(index);
            }
        }
        let index = best?;
        let start = self.free[index].start;
        self.free[index].start = self.free[index].start.checked_add(pages.checked_mul(FRAME_SIZE)?)?;
        self.free[index].pages -= pages;
        if self.free[index].pages == 0 { self.remove_extent(index); }
        self.free_pages -= pages;
        Some(start)
    }

    /// Returns physical pages to the allocator. Double frees and overlapping
    /// returns are rejected before allocator metadata is changed.
    pub fn deallocate_contiguous(&mut self, start: u64, pages: u64) -> Result<(), &'static str> {
        if start == 0 || start % FRAME_SIZE != 0 || pages == 0 {
            return Err("invalid physical frame return");
        }
        let returned = Extent { start, pages };
        let returned_end = returned.end().ok_or("returned frame range overflow")?;
        for index in 0..self.free_count {
            let existing = self.free[index];
            let existing_end = existing.end().ok_or("allocator extent overflow")?;
            if start < existing_end && existing.start < returned_end {
                return Err("physical frame range overlaps free memory");
            }
        }
        self.insert_extent(returned)?;
        self.free_pages = self.free_pages.checked_add(pages).ok_or("free page counter overflow")?;
        self.coalesce_all();
        Ok(())
    }

    pub fn deallocate_frame(&mut self, start: u64) -> Result<(), &'static str> {
        self.deallocate_contiguous(start, 1)
    }

    #[must_use] pub const fn total_pages(&self) -> u64 { self.total_pages }
    #[must_use] pub const fn free_pages(&self) -> u64 { self.free_pages }
    #[must_use] pub const fn allocated_pages(&self) -> u64 { self.total_pages - self.free_pages }

    fn insert_extent(&mut self, extent: Extent) -> Result<(), &'static str> {
        if self.free_count == MAX_FREE_EXTENTS { return Err("physical allocator extent table is full"); }
        let mut index = self.free_count;
        while index > 0 && self.free[index - 1].start > extent.start {
            self.free[index] = self.free[index - 1];
            index -= 1;
        }
        self.free[index] = extent;
        self.free_count += 1;
        Ok(())
    }

    fn remove_extent(&mut self, index: usize) {
        for cursor in index..self.free_count.saturating_sub(1) {
            self.free[cursor] = self.free[cursor + 1];
        }
        self.free_count -= 1;
        self.free[self.free_count] = Extent::EMPTY;
    }

    fn coalesce_all(&mut self) {
        let mut index = 0;
        while index + 1 < self.free_count {
            let current = self.free[index];
            if current.end() == Some(self.free[index + 1].start) {
                self.free[index].pages = self.free[index].pages.saturating_add(self.free[index + 1].pages);
                self.remove_extent(index + 1);
            } else {
                index += 1;
            }
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct MemorySummary { pub total_pages: u64, pub conventional_pages: u64 }

pub fn summarize(boot_info: &BootInfo) -> MemorySummary {
    let mut summary = MemorySummary::default();
    for index in 0..boot_info.memory_map.descriptor_count as usize {
        let Some(descriptor) = read_descriptor(&boot_info.memory_map, index) else { break };
        summary.total_pages = summary.total_pages.saturating_add(descriptor.page_count);
        if descriptor.memory_type == uefi_memory_type::CONVENTIONAL {
            summary.conventional_pages = summary.conventional_pages.saturating_add(descriptor.page_count);
        }
    }
    summary
}

pub fn bytes_to_mib(bytes: u64) -> u64 { bytes / (1024 * 1024) }

fn read_descriptor(map: &MemoryMapInfo, index: usize) -> Option<UefiMemoryDescriptorPrefix> {
    let stride = usize::try_from(map.descriptor_size).ok()?;
    let map_size = usize::try_from(map.buffer_size).ok()?;
    let offset = index.checked_mul(stride)?;
    if offset.checked_add(size_of::<UefiMemoryDescriptorPrefix>())? > map_size { return None; }
    Some(unsafe { ptr::read_unaligned((map.buffer_address as *const u8).add(offset).cast()) })
}

/// Bootstrap arena retained only for very early metadata. Unlike physical
/// frames, arena allocations are released as a group by `reset`.
pub struct BumpHeap { start: u64, end: u64, next: u64 }
impl BumpHeap {
    pub fn new(start: u64, size: u64) -> Option<Self> {
        Some(Self { start, end: start.checked_add(size)?, next: start })
    }
    pub fn allocate(&mut self, size: u64, alignment: u64) -> Option<u64> {
        if size == 0 || !alignment.is_power_of_two() { return None; }
        let aligned = align_up(self.next, alignment)?;
        let end = aligned.checked_add(size)?;
        if end > self.end { return None; }
        self.next = end;
        Some(aligned)
    }
    pub fn reset(&mut self) { self.next = self.start; }
    pub fn capacity(&self) -> u64 { self.end - self.start }
    pub fn used(&self) -> u64 { self.next - self.start }
}
fn align_up(value: u64, alignment: u64) -> Option<u64> {
    value.checked_add(alignment - 1).map(|rounded| rounded & !(alignment - 1))
}
