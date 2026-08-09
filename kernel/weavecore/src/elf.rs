use crate::memory::FrameAllocator;
use crate::paging::AddressSpace;

const ELF_HEADER_SIZE: usize = 64;
const PROGRAM_HEADER_SIZE: usize = 56;
const PT_LOAD: u32 = 1;
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const ET_EXEC: u16 = 2;
const EM_X86_64: u16 = 62;
const MAX_LOAD_SEGMENTS: usize = 8;

pub struct LoadedUserElf {
    pub address_space: AddressSpace,
    pub load_segments: usize,
}

pub fn load_user_elf(
    image: &[u8],
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
) -> Result<LoadedUserElf, &'static str> {
    if image.len() < ELF_HEADER_SIZE || &image[..4] != b"\x7fELF" {
        return Err("user module is not ELF");
    }
    if image[4] != 2 || image[5] != 1 || image[6] != 1 {
        return Err("user ELF must be little-endian ELF64 v1");
    }
    if read_u16(image, 16)? != ET_EXEC || read_u16(image, 18)? != EM_X86_64 {
        return Err("user ELF must be x86-64 ET_EXEC");
    }

    let entry = read_u64(image, 24)?;
    let phoff = usize::try_from(read_u64(image, 32)?).map_err(|_| "ELF PH offset overflow")?;
    let phentsize = read_u16(image, 54)? as usize;
    let phnum = read_u16(image, 56)? as usize;
    if phentsize < PROGRAM_HEADER_SIZE || phnum == 0 {
        return Err("user ELF has no program headers");
    }

    let mut address_space = AddressSpace::create_empty(allocator, kernel_cr3)?;
    let mut load_segments = 0usize;

    for index in 0..phnum {
        let header = phoff
            .checked_add(index.checked_mul(phentsize).ok_or("ELF PH multiplication overflow")?)
            .ok_or("ELF PH offset overflow")?;
        if header.checked_add(PROGRAM_HEADER_SIZE).ok_or("ELF PH bounds overflow")? > image.len() {
            return Err("user ELF program header lies outside the file");
        }
        if read_u32(image, header)? != PT_LOAD {
            continue;
        }
        if load_segments == MAX_LOAD_SEGMENTS {
            return Err("user ELF has too many load segments");
        }

        let flags = read_u32(image, header + 4)?;
        let file_offset = read_u64(image, header + 8)?;
        let virtual_address = read_u64(image, header + 16)?;
        let file_size = read_u64(image, header + 32)?;
        let memory_size = read_u64(image, header + 40)?;
        if memory_size == 0 || file_size > memory_size {
            return Err("user ELF PT_LOAD sizes are invalid");
        }
        let source_start = usize::try_from(file_offset).map_err(|_| "user ELF offset overflow")?;
        let source_end = usize::try_from(
            file_offset.checked_add(file_size).ok_or("user ELF file range overflow")?,
        )
        .map_err(|_| "user ELF file range overflow")?;
        if source_end > image.len() {
            return Err("user ELF segment data lies outside the file");
        }

        address_space.map_segment(
            allocator,
            virtual_address,
            memory_size,
            flags & PF_W != 0,
            flags & PF_X != 0,
        )?;
        if source_end != source_start {
            address_space.copy_image_bytes(virtual_address, &image[source_start..source_end])?;
        }
        load_segments += 1;
    }

    if load_segments == 0 {
        return Err("user ELF has no PT_LOAD segment");
    }
    address_space.allocate_stack(allocator)?;
    address_space.set_entry_point(entry)?;

    Ok(LoadedUserElf {
        address_space,
        load_segments,
    })
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
