use crate::memory::{FrameAllocator, FRAME_SIZE};
use crate::sync::SpinLock;
use core::arch::asm;
use core::cmp::min;
use core::ptr;

const ENTRY_COUNT: usize = 512;
const PAGE_PRESENT: u64 = 1 << 0;
const PAGE_WRITABLE: u64 = 1 << 1;
const PAGE_USER: u64 = 1 << 2;
const PAGE_WRITE_THROUGH: u64 = 1 << 3;
const PAGE_CACHE_DISABLE: u64 = 1 << 4;
const PAGE_HUGE: u64 = 1 << 7;
const PAGE_NO_EXECUTE: u64 = 1 << 63;
const ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;
const MAX_REGIONS: usize = 64;

pub const USER_IMAGE_MIN: u64 = 0x0000_0100_0000_0000;
pub const USER_IMAGE_MAX: u64 = 0x0000_0100_0080_0000;
pub const USER_STACK_TOP: u64 = USER_IMAGE_MAX;
pub const USER_STACK_PAGES: u64 = 8;
pub const USER_STACK_SIZE: u64 = USER_STACK_PAGES * FRAME_SIZE;
pub const USER_STACK_BASE: u64 = USER_STACK_TOP - USER_STACK_SIZE;

/// Dedicated supervisor-only virtual aperture for PCI/MMIO mappings that are
/// not covered by TitanBoot's bounded bootstrap identity map.  This lives in
/// the already-shared kernel PML4 slot so address spaces created after boot
/// inherit the same supervisor mappings without granting PAGE_USER access.
pub const KERNEL_MMIO_BASE: u64 = 0xffff_ffc0_0000_0000;
pub const KERNEL_MMIO_LIMIT: u64 = KERNEL_MMIO_BASE + (1u64 << 30);
static NEXT_KERNEL_MMIO: SpinLock<u64> = SpinLock::new(KERNEL_MMIO_BASE);

/// Map a physical device-MMIO range into Titanweave's supervisor-only,
/// uncached kernel MMIO aperture.  This is intentionally separate from the
/// broad bootstrap identity map: PCI BAR placement is firmware/platform
/// policy and may legally sit above that early direct-map ceiling.
pub fn map_kernel_mmio(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
    physical_address: u64,
    length: u64,
) -> Result<u64, &'static str> {
    if physical_address == 0 || length == 0 {
        return Err("invalid kernel MMIO mapping request");
    }
    let physical_end = physical_address
        .checked_add(length)
        .ok_or("kernel MMIO physical range overflow")?;
    let physical_page = align_down(physical_address, FRAME_SIZE);
    let mapped_end = align_up(physical_end, FRAME_SIZE)
        .ok_or("kernel MMIO page alignment overflow")?;
    let mapped_bytes = mapped_end
        .checked_sub(physical_page)
        .ok_or("kernel MMIO mapped length underflow")?;
    let page_offset = physical_address - physical_page;

    let virtual_page = {
        let mut next = NEXT_KERNEL_MMIO.lock();
        let start = align_up(*next, FRAME_SIZE).ok_or("kernel MMIO virtual alignment overflow")?;
        let end = start
            .checked_add(mapped_bytes)
            .ok_or("kernel MMIO virtual range overflow")?;
        if end > KERNEL_MMIO_LIMIT {
            return Err("kernel MMIO aperture exhausted");
        }
        *next = end;
        start
    };

    let mut physical = physical_page;
    let mut virtual_address = virtual_page;
    while physical < mapped_end {
        map_kernel_mmio_page(allocator, kernel_cr3, virtual_address, physical, true)?;
        physical += FRAME_SIZE;
        virtual_address += FRAME_SIZE;
    }
    virtual_page
        .checked_add(page_offset)
        .ok_or("kernel MMIO result address overflow")
}

/// Map device MMIO into a supervisor-only, uncached, NX *read-only* aperture.
/// C7 uses this to establish a mapping contract without permitting CPU stores.
pub fn map_kernel_mmio_readonly(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
    physical_address: u64,
    length: u64,
) -> Result<u64, &'static str> {
    if physical_address == 0 || length == 0 { return Err("invalid read-only kernel MMIO mapping request"); }
    let physical_end = physical_address.checked_add(length).ok_or("read-only kernel MMIO physical range overflow")?;
    let physical_page = align_down(physical_address, FRAME_SIZE);
    let mapped_end = align_up(physical_end, FRAME_SIZE).ok_or("read-only kernel MMIO page alignment overflow")?;
    let mapped_bytes = mapped_end.checked_sub(physical_page).ok_or("read-only kernel MMIO mapped length underflow")?;
    let page_offset = physical_address - physical_page;
    let virtual_page = {
        let mut next = NEXT_KERNEL_MMIO.lock();
        let start = align_up(*next, FRAME_SIZE).ok_or("read-only kernel MMIO virtual alignment overflow")?;
        let end = start.checked_add(mapped_bytes).ok_or("read-only kernel MMIO virtual range overflow")?;
        if end > KERNEL_MMIO_LIMIT { return Err("kernel MMIO aperture exhausted"); }
        *next = end; start
    };
    let mut physical = physical_page;
    let mut virtual_address = virtual_page;
    while physical < mapped_end {
        map_kernel_mmio_page(allocator, kernel_cr3, virtual_address, physical, false)?;
        physical += FRAME_SIZE; virtual_address += FRAME_SIZE;
    }
    virtual_page.checked_add(page_offset).ok_or("read-only kernel MMIO result address overflow")
}

fn map_kernel_mmio_page(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
    virtual_address: u64,
    physical_address: u64,
    writable: bool,
) -> Result<(), &'static str> {
    if virtual_address < KERNEL_MMIO_BASE || virtual_address >= KERNEL_MMIO_LIMIT {
        return Err("kernel MMIO virtual address outside aperture");
    }
    if physical_address & (FRAME_SIZE - 1) != 0 {
        return Err("kernel MMIO physical page is unaligned");
    }

    let pml4 = kernel_cr3 & ADDRESS_MASK;
    let pdpt = ensure_kernel_table(allocator, pml4, pml4_index(virtual_address))?;
    let pd = ensure_kernel_table(allocator, pdpt, pdpt_index(virtual_address))?;
    let pt = ensure_kernel_table(allocator, pd, pd_index(virtual_address))?;
    let index = pt_index(virtual_address);
    if read_entry(pt, index) & PAGE_PRESENT != 0 {
        return Err("attempted to remap kernel MMIO page");
    }
    let mut flags = PAGE_PRESENT | PAGE_WRITE_THROUGH | PAGE_CACHE_DISABLE | PAGE_NO_EXECUTE;
    if writable { flags |= PAGE_WRITABLE; }
    write_entry(pt, index, physical_address | flags);
    // SAFETY: the mapping belongs to the active kernel page tables.  INVLPG is
    // harmless when no stale translation exists and makes remapping semantics
    // explicit for later hotplug work.
    unsafe { asm!("invlpg [{}]", in(reg) virtual_address, options(nostack, preserves_flags)) };
    Ok(())
}

fn ensure_kernel_table(
    allocator: &mut FrameAllocator<'_>,
    parent: u64,
    index: usize,
) -> Result<u64, &'static str> {
    let existing = read_entry(parent, index);
    if existing & PAGE_PRESENT != 0 {
        if existing & PAGE_HUGE != 0 {
            return Err("kernel MMIO mapping collides with huge page");
        }
        return Ok(existing & ADDRESS_MASK);
    }
    let table = allocate_zeroed(allocator)?;
    write_entry(parent, index, table | PAGE_PRESENT | PAGE_WRITABLE);
    Ok(table)
}

#[derive(Clone, Copy)]
struct MemoryRegion {
    start: u64,
    end: u64,
    writable: bool,
    executable: bool,
}

impl MemoryRegion {
    const EMPTY: Self = Self {
        start: 0,
        end: 0,
        writable: false,
        executable: false,
    };

    fn contains(&self, address: u64, length: usize) -> bool {
        if self.start == self.end || length == 0 {
            return false;
        }
        let Some(end) = address.checked_add(length as u64) else {
            return false;
        };
        address >= self.start && end <= self.end
    }
}

#[derive(Clone, Copy)]
pub struct AddressSpace {
    pub cr3: u64,
    pub entry_point: u64,
    pub stack_virtual_base: u64,
    pub stack_size: u64,
    regions: [MemoryRegion; MAX_REGIONS],
    region_count: usize,
}

impl AddressSpace {
    pub fn create_empty(
        allocator: &mut FrameAllocator<'_>,
        kernel_cr3: u64,
    ) -> Result<Self, &'static str> {
        let pml4 = allocate_zeroed(allocator)?;
        unsafe {
            ptr::copy_nonoverlapping(
                (kernel_cr3 & ADDRESS_MASK) as *const u64,
                pml4 as *mut u64,
                ENTRY_COUNT,
            );
        }
        // The entire user-image slot must begin empty. The bootstrap identity
        // map uses PML4[0]; Titanweave user images use PML4[2].
        write_entry(pml4, pml4_index(USER_IMAGE_MIN), 0);

        Ok(Self {
            cr3: pml4,
            entry_point: 0,
            stack_virtual_base: 0,
            stack_size: 0,
            regions: [MemoryRegion::EMPTY; MAX_REGIONS],
            region_count: 0,
        })
    }

    pub fn map_segment(
        &mut self,
        allocator: &mut FrameAllocator<'_>,
        virtual_address: u64,
        memory_size: u64,
        writable: bool,
        executable: bool,
    ) -> Result<(), &'static str> {
        if memory_size == 0 {
            return Err("user ELF segment has zero memory size");
        }
        let end = virtual_address
            .checked_add(memory_size)
            .ok_or("user segment address overflow")?;
        if virtual_address < USER_IMAGE_MIN || end > USER_STACK_BASE {
            return Err("user ELF segment lies outside the application window");
        }

        let page_start = align_down(virtual_address, FRAME_SIZE);
        let page_end = align_up(end, FRAME_SIZE).ok_or("user segment alignment overflow")?;
        let mut page = page_start;
        while page < page_end {
            if self.translate(page).is_some() {
                return Err("overlapping user ELF segments are not supported in K6");
            }
            let frame = allocate_zeroed(allocator)?;
            self.map_page(allocator, page, frame, writable, executable)?;
            page += FRAME_SIZE;
        }
        self.add_region(page_start, page_end, writable, executable)
    }

    pub fn allocate_stack(&mut self, allocator: &mut FrameAllocator<'_>) -> Result<(), &'static str> {
        let mut page = USER_STACK_BASE;
        while page < USER_STACK_TOP {
            let frame = allocate_zeroed(allocator)?;
            self.map_page(allocator, page, frame, true, false)?;
            page += FRAME_SIZE;
        }
        self.stack_virtual_base = USER_STACK_BASE;
        self.stack_size = USER_STACK_SIZE;
        self.add_region(USER_STACK_BASE, USER_STACK_TOP, true, false)
    }

    pub fn set_entry_point(&mut self, entry_point: u64) -> Result<(), &'static str> {
        if !self
            .regions[..self.region_count]
            .iter()
            .any(|region| region.executable && region.contains(entry_point, 1))
        {
            return Err("user ELF entry point is not executable");
        }
        self.entry_point = entry_point;
        Ok(())
    }

    #[must_use]
    pub fn permits_read(&self, address: u64, length: usize) -> bool {
        self.regions[..self.region_count]
            .iter()
            .any(|region| region.contains(address, length))
    }

    #[must_use]
    pub fn permits_write(&self, address: u64, length: usize) -> bool {
        self.regions[..self.region_count]
            .iter()
            .any(|region| region.writable && region.contains(address, length))
    }

    pub fn copy_from_user(&self, address: u64, output: &mut [u8]) -> Result<(), &'static str> {
        if !self.permits_read(address, output.len()) {
            return Err("user read lies outside process mappings");
        }
        self.copy_from_mapped(address, output)
    }

    pub fn copy_to_user(&self, address: u64, input: &[u8]) -> Result<(), &'static str> {
        if !self.permits_write(address, input.len()) {
            return Err("user write lies outside writable process mappings");
        }
        self.copy_to_mapped(address, input)
    }

    pub fn copy_image_bytes(&self, address: u64, input: &[u8]) -> Result<(), &'static str> {
        if !self.permits_read(address, input.len()) {
            return Err("ELF file bytes lie outside a mapped segment");
        }
        self.copy_to_mapped(address, input)
    }

    fn add_region(
        &mut self,
        start: u64,
        end: u64,
        writable: bool,
        executable: bool,
    ) -> Result<(), &'static str> {
        if self.region_count == MAX_REGIONS {
            return Err("user address-space region table is full");
        }
        self.regions[self.region_count] = MemoryRegion {
            start,
            end,
            writable,
            executable,
        };
        self.region_count += 1;
        Ok(())
    }

    fn map_page(
        &self,
        allocator: &mut FrameAllocator<'_>,
        virtual_address: u64,
        physical_address: u64,
        writable: bool,
        executable: bool,
    ) -> Result<(), &'static str> {
        let pml4 = self.cr3 & ADDRESS_MASK;
        let pdpt = ensure_table(allocator, pml4, pml4_index(virtual_address))?;
        let pd = ensure_table(allocator, pdpt, pdpt_index(virtual_address))?;
        let pt = ensure_table(allocator, pd, pd_index(virtual_address))?;

        let mut flags = PAGE_PRESENT | PAGE_USER;
        if writable {
            flags |= PAGE_WRITABLE;
        }
        if !executable {
            flags |= PAGE_NO_EXECUTE;
        }
        let index = pt_index(virtual_address);
        if read_entry(pt, index) & PAGE_PRESENT != 0 {
            return Err("attempted to remap an existing user page");
        }
        write_entry(pt, index, physical_address | flags);
        Ok(())
    }

    #[must_use]
    pub fn translate(&self, virtual_address: u64) -> Option<u64> {
        let pml4e = read_entry(self.cr3 & ADDRESS_MASK, pml4_index(virtual_address));
        if pml4e & PAGE_PRESENT == 0 {
            return None;
        }
        let pdpte = read_entry(pml4e & ADDRESS_MASK, pdpt_index(virtual_address));
        if pdpte & PAGE_PRESENT == 0 {
            return None;
        }
        let pde = read_entry(pdpte & ADDRESS_MASK, pd_index(virtual_address));
        if pde & PAGE_PRESENT == 0 {
            return None;
        }
        let pte = read_entry(pde & ADDRESS_MASK, pt_index(virtual_address));
        if pte & PAGE_PRESENT == 0 {
            return None;
        }
        Some((pte & ADDRESS_MASK) + (virtual_address & (FRAME_SIZE - 1)))
    }

    fn copy_from_mapped(&self, mut address: u64, mut output: &mut [u8]) -> Result<(), &'static str> {
        while !output.is_empty() {
            let physical = self.translate(address).ok_or("unmapped user read")?;
            let chunk = min(output.len(), (FRAME_SIZE - (address & (FRAME_SIZE - 1))) as usize);
            unsafe { ptr::copy_nonoverlapping(physical as *const u8, output.as_mut_ptr(), chunk) };
            address += chunk as u64;
            output = &mut output[chunk..];
        }
        Ok(())
    }

    fn copy_to_mapped(&self, mut address: u64, mut input: &[u8]) -> Result<(), &'static str> {
        while !input.is_empty() {
            let physical = self.translate(address).ok_or("unmapped user write")?;
            let chunk = min(input.len(), (FRAME_SIZE - (address & (FRAME_SIZE - 1))) as usize);
            unsafe { ptr::copy_nonoverlapping(input.as_ptr(), physical as *mut u8, chunk) };
            address += chunk as u64;
            input = &input[chunk..];
        }
        Ok(())
    }

    /// Reclaims every user page and page-table page owned by this address space.
    /// Kernel PML4 mappings are copied from the bootstrap CR3 and are never freed.
    pub fn destroy(&mut self, allocator: &mut FrameAllocator<'_>) -> Result<u64, &'static str> {
        let mut released = 0u64;
        let user_index = pml4_index(USER_IMAGE_MIN);
        let pml4 = self.cr3 & ADDRESS_MASK;
        let pml4e = read_entry(pml4, user_index);
        if pml4e & PAGE_PRESENT != 0 {
            let pdpt = pml4e & ADDRESS_MASK;
            for pdpt_i in 0..ENTRY_COUNT {
                let pdpte = read_entry(pdpt, pdpt_i);
                if pdpte & PAGE_PRESENT == 0 { continue; }
                let pd = pdpte & ADDRESS_MASK;
                for pd_i in 0..ENTRY_COUNT {
                    let pde = read_entry(pd, pd_i);
                    if pde & PAGE_PRESENT == 0 { continue; }
                    let pt = pde & ADDRESS_MASK;
                    for pt_i in 0..ENTRY_COUNT {
                        let pte = read_entry(pt, pt_i);
                        if pte & PAGE_PRESENT != 0 { allocator.deallocate_frame(pte & ADDRESS_MASK)?; write_entry(pt, pt_i, 0); released += 1; }
                    }
                    allocator.deallocate_frame(pt)?; write_entry(pd, pd_i, 0); released += 1;
                }
                allocator.deallocate_frame(pd)?; write_entry(pdpt, pdpt_i, 0); released += 1;
            }
            allocator.deallocate_frame(pdpt)?; write_entry(pml4, user_index, 0); released += 1;
        }
        allocator.deallocate_frame(pml4)?; released += 1;
        self.cr3 = 0; self.entry_point = 0; self.stack_virtual_base = 0; self.stack_size = 0; self.regions = [MemoryRegion::EMPTY; MAX_REGIONS]; self.region_count = 0;
        Ok(released)
    }
}

fn ensure_table(
    allocator: &mut FrameAllocator<'_>,
    parent: u64,
    index: usize,
) -> Result<u64, &'static str> {
    let existing = read_entry(parent, index);
    if existing & PAGE_PRESENT != 0 {
        return Ok(existing & ADDRESS_MASK);
    }
    let table = allocate_zeroed(allocator)?;
    write_entry(
        parent,
        index,
        table | PAGE_PRESENT | PAGE_WRITABLE | PAGE_USER,
    );
    Ok(table)
}

fn allocate_zeroed(allocator: &mut FrameAllocator<'_>) -> Result<u64, &'static str> {
    let frame = allocator
        .allocate_frame()
        .ok_or("no physical frame available for user address space")?;
    unsafe { ptr::write_bytes(frame as *mut u8, 0, FRAME_SIZE as usize) };
    Ok(frame)
}

fn read_entry(table: u64, index: usize) -> u64 {
    assert!(index < ENTRY_COUNT);
    unsafe { ptr::read((table as *const u64).add(index)) }
}

fn write_entry(table: u64, index: usize, value: u64) {
    assert!(index < ENTRY_COUNT);
    unsafe { ptr::write((table as *mut u64).add(index), value) };
}

const fn pml4_index(address: u64) -> usize {
    ((address >> 39) & 0x1ff) as usize
}

const fn pdpt_index(address: u64) -> usize {
    ((address >> 30) & 0x1ff) as usize
}

const fn pd_index(address: u64) -> usize {
    ((address >> 21) & 0x1ff) as usize
}

const fn pt_index(address: u64) -> usize {
    ((address >> 12) & 0x1ff) as usize
}

const fn align_down(value: u64, alignment: u64) -> u64 {
    value & !(alignment - 1)
}

fn align_up(value: u64, alignment: u64) -> Option<u64> {
    value
        .checked_add(alignment - 1)
        .map(|rounded| align_down(rounded, alignment))
}
