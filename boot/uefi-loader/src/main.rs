#![no_main]
#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::arch::asm;
use core::mem;
use core::panic::PanicInfo;
use core::ptr::{self, NonNull};
use titanweave_boot_protocol::{
    boot_module_kind, AcpiInfo, BootInfo, BootModuleInfo, BootModulesInfo, BootstrapInfo,
    FramebufferInfo, KernelImageInfo, MemoryMapInfo, SmpBootstrapInfo, BOOT_MODULE_NAME_BYTES, UEFI_PAGE_SIZE,
};
use uefi::boot::{AllocateType, MemoryType};
use uefi::fs::FileSystem;
use uefi::proto::console::gop::{GraphicsOutput, PixelFormat};
use uefi::mem::memory_map::MemoryMap;
use uefi::prelude::{entry, Status};
use uefi::table::cfg::ConfigTableEntry;
use uefi::cstr16;

const KERNEL_PATH: &uefi::CStr16 = cstr16!("\\WEAVECORE.ELF");
const BOOT_VOLUME_PATH: &uefi::CStr16 = cstr16!("\\TITANFS.IMG");
const STACK_SIZE: usize = 256 * 1024;
const AP_TRAMPOLINE_PHYSICAL_BASE: u64 = 0x0000_8000;
const AP_TRAMPOLINE_PAGES: usize = 1;
const MAX_LOGICAL_CPUS: u32 = 64;
const BOOT_INFO_PAGES: usize = 1;
const IDENTITY_MAP_GIB: usize = 512;
const PAGE_TABLE_PAGES: usize = 1 + 1 + IDENTITY_MAP_GIB + 1 + 1;
const PAGE_PRESENT: u64 = 1 << 0;
const PAGE_WRITABLE: u64 = 1 << 1;
const PAGE_HUGE: u64 = 1 << 7;
const ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;
const KERNEL_HIGHER_HALF_BASE: u64 = 0xffff_ffff_8000_0000;

#[entry]
fn main() -> Status {
    uefi::helpers::init().expect("UEFI helper initialization failed");
    uefi::println!("TITAN//WEAVE");
    uefi::println!("Titanweave UEFI loader K14");

    let kernel_file = read_boot_file(KERNEL_PATH, "\\WEAVECORE.ELF");
    let loaded_kernel = load_elf64(&kernel_file).expect("Invalid or unloadable kernel ELF");
    uefi::println!(
        "Loaded WEAVECORE.ELF: entry={:#x}, physical={:#x}..{:#x}",
        loaded_kernel.entry_point,
        loaded_kernel.physical_base,
        loaded_kernel.physical_base + loaded_kernel.physical_size
    );

    let mut boot_modules = BootModulesInfo::empty();
    boot_modules
        .push(load_boot_module(
            BOOT_VOLUME_PATH,
            "titanfs",
            boot_module_kind::BOOT_VOLUME,
        ))
        .expect("Failed to record TitanFS bootstrap volume");
    uefi::println!(
        "Preloaded TitanFS bootstrap volume as {} boot module",
        boot_modules.count
    );

    let stack = allocate_below_4g(STACK_SIZE / UEFI_PAGE_SIZE as usize)
        .expect("Failed to allocate kernel bootstrap stack");
    zero_pages(stack, STACK_SIZE / UEFI_PAGE_SIZE as usize);

    let boot_info_page = allocate_below_4g(BOOT_INFO_PAGES)
        .expect("Failed to allocate BootInfo page");
    zero_pages(boot_info_page, BOOT_INFO_PAGES);

    let ap_trampoline = uefi::boot::allocate_pages(
        AllocateType::Address(AP_TRAMPOLINE_PHYSICAL_BASE),
        MemoryType::LOADER_DATA,
        AP_TRAMPOLINE_PAGES,
    )
    .expect("Failed to reserve the low-memory AP trampoline page at 0x8000");
    zero_pages(ap_trampoline, AP_TRAMPOLINE_PAGES);

    let rsdp_address = find_rsdp_address()
        .expect("UEFI did not expose an ACPI RSDP; K6 requires ACPI");
    uefi::println!("ACPI RSDP located at {:#x}", rsdp_address);

    let page_tables = allocate_below_4g(PAGE_TABLE_PAGES)
        .expect("Failed to allocate bootstrap page tables");
    zero_pages(page_tables, PAGE_TABLE_PAGES);

    let cr3 = build_bootstrap_page_tables(page_tables, &loaded_kernel);
    let stack_base = stack.as_ptr() as u64;
    let stack_top = stack_base + STACK_SIZE as u64;
    let boot_info_address = boot_info_page.as_ptr() as u64;

    // K12 captures a standards-compliant UEFI GOP linear framebuffer as the
    // always-available display fallback. Native GPU drivers can replace this
    // scanout after ForgeBus claims the adapter.
    let framebuffer = capture_framebuffer();
    if framebuffer.is_empty() {
        serial_initialize();
        serial_write("[LOAD] GOP linear framebuffer unavailable; K14 will boot headless\n");
    } else {
        serial_initialize();
        serial_write("[LOAD] GOP framebuffer captured for K14 handoff\n");
    }

    // Drop every UEFI protocol and pool-backed object before ExitBootServices.
    drop(kernel_file);

    serial_write("Exiting UEFI boot services and entering WeaveCore...\n");

    // SAFETY: All protocol handles and ordinary UEFI pool allocations owned by
    // this function have been dropped. The page allocations intentionally
    // survive and are represented in the final memory map.
    let memory_map = unsafe { uefi::boot::exit_boot_services(None) };
    let meta = memory_map.meta();
    let buffer = memory_map.buffer();

    let boot_info = BootInfo::new(
        MemoryMapInfo {
            buffer_address: buffer.as_ptr() as u64,
            buffer_size: meta.map_size as u64,
            descriptor_size: meta.desc_size as u64,
            descriptor_version: meta.desc_version,
            descriptor_count: u32::try_from(meta.entry_count()).unwrap_or(u32::MAX),
        },
        KernelImageInfo {
            physical_base: loaded_kernel.physical_base,
            physical_size: loaded_kernel.physical_size,
            virtual_base: loaded_kernel.virtual_base,
            virtual_size: loaded_kernel.virtual_size,
            entry_point: loaded_kernel.entry_point,
        },
        BootstrapInfo {
            page_table_root: cr3,
            stack_physical_base: stack_base,
            stack_virtual_base: stack_base,
            stack_size: STACK_SIZE as u64,
            identity_map_limit: (IDENTITY_MAP_GIB as u64) << 30,
        },
        framebuffer,
        AcpiInfo { rsdp_address },
        SmpBootstrapInfo {
            trampoline_physical_base: AP_TRAMPOLINE_PHYSICAL_BASE,
            trampoline_size: (AP_TRAMPOLINE_PAGES as u64) * UEFI_PAGE_SIZE,
            maximum_logical_cpus: MAX_LOGICAL_CPUS,
            reserved: 0,
        },
        boot_modules,
    );

    // SAFETY: boot_info_page is a reserved, writable page large enough and
    // aligned for BootInfo. It remains identity mapped by the new tables.
    unsafe { ptr::write(boot_info_address as *mut BootInfo, boot_info) };

    // The final memory-map allocation must remain valid for the kernel.
    mem::forget(memory_map);

    // SAFETY: The ELF loader validated the executable, page tables map the
    // loader's low identity region plus the higher-half kernel, and the stack
    // and BootInfo allocations remain reserved.
    unsafe {
        transfer_to_kernel(
            loaded_kernel.entry_point,
            boot_info_address,
            stack_top,
            cr3,
        )
    }
}


const K13_FALLBACK_MAX_WIDTH: usize = 2560;
const K13_FALLBACK_MAX_HEIGHT: usize = 1440;

fn capture_framebuffer() -> FramebufferInfo {
    let Ok(handle) = uefi::boot::get_handle_for_protocol::<GraphicsOutput>() else {
        return FramebufferInfo::empty();
    };
    let Ok(mut gop) = uefi::boot::open_protocol_exclusive::<GraphicsOutput>(handle) else {
        return FramebufferInfo::empty();
    };

    // Prefer the largest direct framebuffer mode that fits Titanweave's
    // conservative early-boot scanout envelope. This avoids BLT-only modes and
    // gives the eventual compositor useful space without depending on a GPU
    // driver during bootstrap.
    let preferred = gop
        .modes()
        .filter(|mode| {
            let info = mode.info();
            let (width, height) = info.resolution();
            matches!(info.pixel_format(), PixelFormat::Rgb | PixelFormat::Bgr)
                && width <= K13_FALLBACK_MAX_WIDTH
                && height <= K13_FALLBACK_MAX_HEIGHT
        })
        .max_by_key(|mode| {
            let (width, height) = mode.info().resolution();
            width.saturating_mul(height)
        });
    if let Some(mode) = preferred {
        let current = gop.current_mode_info();
        if current.resolution() != mode.info().resolution()
            || current.pixel_format() != mode.info().pixel_format()
        {
            let _ = gop.set_mode(&mode);
        }
    }

    let info = gop.current_mode_info();
    let (width, height) = info.resolution();
    let pixel_format = match info.pixel_format() {
        PixelFormat::Rgb => 0,
        PixelFormat::Bgr => 1,
        PixelFormat::Bitmask => 2,
        PixelFormat::BltOnly => 3,
    };
    if pixel_format > 1 || width == 0 || height == 0 || info.stride() < width {
        return FramebufferInfo::empty();
    }

    let mut frame_buffer = gop.frame_buffer();
    let descriptor = FramebufferInfo {
        base_address: frame_buffer.as_mut_ptr() as u64,
        byte_size: frame_buffer.size() as u64,
        width: u32::try_from(width).unwrap_or(u32::MAX),
        height: u32::try_from(height).unwrap_or(u32::MAX),
        stride: u32::try_from(info.stride()).unwrap_or(u32::MAX),
        pixel_format,
    };
    if descriptor.is_structurally_valid_or_empty() {
        descriptor
    } else {
        FramebufferInfo::empty()
    }
}


fn find_rsdp_address() -> Option<u64> {
    let mut acpi_v1 = None;
    let mut acpi_v2 = None;

    uefi::system::with_config_table(|tables| {
        for entry in tables {
            if entry.guid == ConfigTableEntry::ACPI2_GUID {
                acpi_v2 = Some(entry.address as usize as u64);
            } else if entry.guid == ConfigTableEntry::ACPI_GUID {
                acpi_v1 = Some(entry.address as usize as u64);
            }
        }
    });

    acpi_v2.or(acpi_v1)
}

fn read_boot_file(path: &uefi::CStr16, description: &str) -> Vec<u8> {
    let fs = uefi::boot::get_image_file_system(uefi::boot::image_handle())
        .expect("Failed to open the boot volume");
    let mut fs = FileSystem::new(fs);
    fs.read(path).unwrap_or_else(|_| panic!("Failed to read {description} from the boot volume"))
}

fn load_boot_module(path: &uefi::CStr16, name: &str, kind: u32) -> BootModuleInfo {
    let file = read_boot_file(path, name);
    assert!(!file.is_empty(), "boot module must not be empty");
    let pages = (file.len() + UEFI_PAGE_SIZE as usize - 1) / UEFI_PAGE_SIZE as usize;
    let allocation = allocate_below_4g(pages).expect("Failed to allocate boot-module pages");
    zero_pages(allocation, pages);
    unsafe { ptr::copy_nonoverlapping(file.as_ptr(), allocation.as_ptr(), file.len()) };

    let mut module_name = [0u8; BOOT_MODULE_NAME_BYTES];
    let bytes = name.as_bytes();
    assert!(bytes.len() < BOOT_MODULE_NAME_BYTES, "boot-module name is too long");
    module_name[..bytes.len()].copy_from_slice(bytes);

    BootModuleInfo {
        name: module_name,
        kind,
        flags: 0,
        physical_address: allocation.as_ptr() as u64,
        byte_size: file.len() as u64,
        entry_hint: 0,
    }
}

#[derive(Clone, Copy)]
struct LoadedKernel {
    entry_point: u64,
    physical_base: u64,
    physical_size: u64,
    virtual_base: u64,
    virtual_size: u64,
}

fn load_elf64(file: &[u8]) -> Result<LoadedKernel, &'static str> {
    if file.len() < 64 || &file[0..4] != b"\x7fELF" {
        return Err("ELF magic missing");
    }
    if file[4] != 2 || file[5] != 1 || file[6] != 1 {
        return Err("Kernel must be little-endian ELF64 v1");
    }
    if read_u16(file, 16)? != 2 || read_u16(file, 18)? != 62 {
        return Err("Kernel must be an x86-64 ET_EXEC image");
    }

    let entry = read_u64(file, 24)?;
    let phoff = usize::try_from(read_u64(file, 32)?).map_err(|_| "PH offset overflow")?;
    let phentsize = read_u16(file, 54)? as usize;
    let phnum = read_u16(file, 56)? as usize;
    if phentsize < 56 || phnum == 0 {
        return Err("No usable program headers");
    }

    let mut physical_base = u64::MAX;
    let mut physical_end = 0u64;
    let mut virtual_base = u64::MAX;
    let mut virtual_end = 0u64;
    let mut load_count = 0usize;

    for index in 0..phnum {
        let header = phoff
            .checked_add(index.checked_mul(phentsize).ok_or("PH multiplication overflow")?)
            .ok_or("PH offset overflow")?;
        if header.checked_add(56).ok_or("PH bounds overflow")? > file.len() {
            return Err("Program header outside ELF file");
        }
        if read_u32(file, header)? != 1 {
            continue;
        }

        let file_offset = read_u64(file, header + 8)?;
        let virtual_address = read_u64(file, header + 16)?;
        let physical_address = read_u64(file, header + 24)?;
        let file_size = read_u64(file, header + 32)?;
        let memory_size = read_u64(file, header + 40)?;

        if memory_size == 0 || file_size > memory_size {
            return Err("Invalid PT_LOAD sizes");
        }
        if load_count != 0 {
            return Err("K6 kernel loader requires exactly one PT_LOAD segment");
        }
        if (virtual_address & (UEFI_PAGE_SIZE - 1))
            != (physical_address & (UEFI_PAGE_SIZE - 1))
        {
            return Err("ELF virtual and physical page offsets differ");
        }
        if virtual_address < KERNEL_HIGHER_HALF_BASE {
            return Err("Kernel segment is not in the higher half");
        }

        let segment_start = align_down(physical_address, UEFI_PAGE_SIZE);
        let segment_end = align_up(
            physical_address
                .checked_add(memory_size)
                .ok_or("Segment address overflow")?,
            UEFI_PAGE_SIZE,
        )?;
        if segment_end > ((IDENTITY_MAP_GIB as u64) << 30) {
            return Err("Kernel physical image lies outside the bootstrap identity map");
        }
        let pages = usize::try_from((segment_end - segment_start) / UEFI_PAGE_SIZE)
            .map_err(|_| "Segment page count overflow")?;

        let allocation = uefi::boot::allocate_pages(
            AllocateType::Address(segment_start),
            MemoryType::LOADER_DATA,
            pages,
        )
        .map_err(|_| "UEFI could not reserve the kernel physical range")?;
        zero_pages(allocation, pages);

        let source_start = usize::try_from(file_offset).map_err(|_| "File offset overflow")?;
        let source_end = usize::try_from(
            file_offset
                .checked_add(file_size)
                .ok_or("File range overflow")?,
        )
        .map_err(|_| "File range overflow")?;
        if source_end > file.len() {
            return Err("PT_LOAD data outside ELF file");
        }

        // SAFETY: The physical range was reserved above, the destination range
        // is within p_memsz, and the source was bounds checked.
        unsafe {
            ptr::copy_nonoverlapping(
                file[source_start..source_end].as_ptr(),
                allocation
                    .as_ptr()
                    .add(usize::try_from(physical_address - segment_start).unwrap()),
                source_end - source_start,
            );
        }

        physical_base = physical_base.min(segment_start);
        physical_end = physical_end.max(segment_end);
        virtual_base = virtual_base.min(align_down(virtual_address, UEFI_PAGE_SIZE));
        virtual_end = virtual_end.max(align_up(
            virtual_address
                .checked_add(memory_size)
                .ok_or("Virtual segment overflow")?,
            UEFI_PAGE_SIZE,
        )?);
        load_count += 1;
    }

    if load_count == 0 || entry < virtual_base || entry >= virtual_end {
        return Err("Kernel has no valid load segment or entry point");
    }

    Ok(LoadedKernel {
        entry_point: entry,
        physical_base,
        physical_size: physical_end - physical_base,
        virtual_base,
        virtual_size: virtual_end - virtual_base,
    })
}

fn build_bootstrap_page_tables(base: NonNull<u8>, kernel: &LoadedKernel) -> u64 {
    let pool = base.as_ptr() as u64;
    let pml4 = pool;
    let low_pdpt = pool + UEFI_PAGE_SIZE;
    let low_pd_base = low_pdpt + UEFI_PAGE_SIZE;
    let high_pdpt = low_pd_base + (IDENTITY_MAP_GIB as u64 * UEFI_PAGE_SIZE);
    let high_pd = high_pdpt + UEFI_PAGE_SIZE;

    set_table_entry(pml4, 0, low_pdpt | PAGE_PRESENT | PAGE_WRITABLE);
    for gib in 0..IDENTITY_MAP_GIB {
        let pd = low_pd_base + gib as u64 * UEFI_PAGE_SIZE;
        set_table_entry(low_pdpt, gib, pd | PAGE_PRESENT | PAGE_WRITABLE);
        for entry in 0..512usize {
            let physical = ((gib as u64) << 30) + ((entry as u64) << 21);
            set_table_entry(
                pd,
                entry,
                physical | PAGE_PRESENT | PAGE_WRITABLE | PAGE_HUGE,
            );
        }
    }

    let pml4_index = index_for(kernel.virtual_base, 39);
    let pdpt_index = index_for(kernel.virtual_base, 30);
    set_table_entry(pml4, pml4_index, high_pdpt | PAGE_PRESENT | PAGE_WRITABLE);
    set_table_entry(
        high_pdpt,
        pdpt_index,
        high_pd | PAGE_PRESENT | PAGE_WRITABLE,
    );

    let first_virtual_2m = align_down(kernel.virtual_base, 2 * 1024 * 1024);
    let first_physical_2m = align_down(kernel.physical_base, 2 * 1024 * 1024);
    let bytes_to_map = (kernel.virtual_base - first_virtual_2m)
        .saturating_add(kernel.virtual_size);
    let huge_pages = ((bytes_to_map + (2 * 1024 * 1024 - 1)) / (2 * 1024 * 1024)) as usize;
    let first_pd_index = index_for(first_virtual_2m, 21);

    for page in 0..huge_pages {
        let index = first_pd_index + page;
        assert!(index < 512, "K6 kernel crosses a 1 GiB page-directory boundary");
        let physical = first_physical_2m + (page as u64 * 2 * 1024 * 1024);
        set_table_entry(
            high_pd,
            index,
            physical | PAGE_PRESENT | PAGE_WRITABLE | PAGE_HUGE,
        );
    }

    pml4 & ADDRESS_MASK
}

fn set_table_entry(table: u64, index: usize, value: u64) {
    assert!(index < 512);
    // SAFETY: The page-table pool was reserved and zeroed; every table address
    // is page aligned and index is checked.
    unsafe { ptr::write((table as *mut u64).add(index), value) };
}

fn index_for(address: u64, shift: u32) -> usize {
    ((address >> shift) & 0x1ff) as usize
}

fn allocate_below_4g(pages: usize) -> uefi::Result<NonNull<u8>> {
    uefi::boot::allocate_pages(
        AllocateType::MaxAddress(0xffff_ffff),
        MemoryType::LOADER_DATA,
        pages,
    )
}

fn zero_pages(base: NonNull<u8>, pages: usize) {
    // SAFETY: Caller owns `pages` UEFI pages at `base`.
    unsafe { ptr::write_bytes(base.as_ptr(), 0, pages * UEFI_PAGE_SIZE as usize) };
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, &'static str> {
    let data = bytes.get(offset..offset + 2).ok_or("ELF read outside file")?;
    Ok(u16::from_le_bytes([data[0], data[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, &'static str> {
    let data = bytes.get(offset..offset + 4).ok_or("ELF read outside file")?;
    Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, &'static str> {
    let data = bytes.get(offset..offset + 8).ok_or("ELF read outside file")?;
    Ok(u64::from_le_bytes([
        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
    ]))
}

const fn align_down(value: u64, alignment: u64) -> u64 {
    value & !(alignment - 1)
}

fn align_up(value: u64, alignment: u64) -> Result<u64, &'static str> {
    value
        .checked_add(alignment - 1)
        .map(|rounded| align_down(rounded, alignment))
        .ok_or("Alignment overflow")
}

unsafe fn transfer_to_kernel(entry: u64, boot_info: u64, stack_top: u64, cr3: u64) -> ! {
    unsafe {
        asm!(
            "cli",
            "mov cr3, {new_cr3}",
            "mov rsp, {new_stack}",
            "and rsp, -16",
            "sub rsp, 8",
            "xor rbp, rbp",
            "mov rdi, {boot_info}",
            "jmp {entry}",
            new_cr3 = in(reg) cr3,
            new_stack = in(reg) stack_top,
            boot_info = in(reg) boot_info,
            entry = in(reg) entry,
            options(noreturn)
        )
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    serial_initialize();
    serial_write("\n[LOAD PANIC] ");
    struct Adapter;
    impl core::fmt::Write for Adapter {
        fn write_str(&mut self, text: &str) -> core::fmt::Result {
            serial_write(text);
            Ok(())
        }
    }
    let _ = core::fmt::write(&mut Adapter, format_args!("{info}\n"));
    loop {
        unsafe { asm!("cli", "hlt", options(nomem, nostack)) };
    }
}

fn serial_initialize() {
    unsafe {
        outb(0x3f9, 0x00);
        outb(0x3fb, 0x80);
        outb(0x3f8, 0x03);
        outb(0x3f9, 0x00);
        outb(0x3fb, 0x03);
        outb(0x3fa, 0xc7);
        outb(0x3fc, 0x0b);
    }
}

fn serial_write(text: &str) {
    for byte in text.bytes() {
        if byte == b'\n' {
            serial_byte(b'\r');
        }
        serial_byte(byte);
    }
}

fn serial_byte(byte: u8) {
    unsafe {
        while inb(0x3fd) & 0x20 == 0 {
            core::hint::spin_loop();
        }
        outb(0x3f8, byte);
    }
}

unsafe fn outb(port: u16, value: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack));
    }
}

unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack));
    }
    value
}
