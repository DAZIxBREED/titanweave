use crate::block::{MemoryBlockDevice, SECTOR_SIZE};

const FAT32_EOC: u32 = 0x0fff_fff8;
const ATTR_DIRECTORY: u8 = 0x10;
const ATTR_LONG_NAME: u8 = 0x0f;
const ATTR_VOLUME_ID: u8 = 0x08;

#[derive(Clone, Copy)]
pub struct DirectoryEntry {
    pub short_name: [u8; 11],
    pub attributes: u8,
    pub first_cluster: u32,
    pub byte_size: u32,
}

impl DirectoryEntry {
    #[must_use]
    pub const fn is_directory(&self) -> bool {
        self.attributes & ATTR_DIRECTORY != 0
    }
}

#[derive(Clone, Copy)]
pub struct Fat32Volume {
    device: MemoryBlockDevice,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    fat_count: u8,
    fat_size_sectors: u32,
    root_cluster: u32,
    first_data_sector: u32,
}

impl Fat32Volume {
    pub fn mount(device: MemoryBlockDevice) -> Result<Self, &'static str> {
        let mut sector = [0u8; SECTOR_SIZE];
        device.read_sector(0, &mut sector)?;
        if sector[510] != 0x55 || sector[511] != 0xaa {
            return Err("FAT32 boot signature is missing");
        }
        let bytes_per_sector = read_u16(&sector, 11)?;
        let sectors_per_cluster = sector[13];
        let reserved_sectors = read_u16(&sector, 14)?;
        let fat_count = sector[16];
        let total_sectors_16 = read_u16(&sector, 19)? as u32;
        let total_sectors_32 = read_u32(&sector, 32)?;
        let fat_size_sectors = read_u32(&sector, 36)?;
        let root_cluster = read_u32(&sector, 44)?;
        let total_sectors = if total_sectors_16 != 0 {
            total_sectors_16
        } else {
            total_sectors_32
        };

        if bytes_per_sector as usize != SECTOR_SIZE {
            return Err("K6 FAT32 supports 512-byte sectors only");
        }
        if sectors_per_cluster == 0 || !sectors_per_cluster.is_power_of_two() {
            return Err("FAT32 sectors-per-cluster is invalid");
        }
        if reserved_sectors == 0 || fat_count == 0 || fat_size_sectors == 0 || root_cluster < 2 {
            return Err("FAT32 geometry is invalid");
        }
        if total_sectors == 0 || total_sectors as u64 > device.sector_count() {
            return Err("FAT32 total-sector count exceeds the block image");
        }

        let first_data_sector = (reserved_sectors as u32)
            .checked_add((fat_count as u32).saturating_mul(fat_size_sectors))
            .ok_or("FAT32 data-sector calculation overflow")?;
        if first_data_sector >= total_sectors {
            return Err("FAT32 data area is empty");
        }
        let data_sectors = total_sectors - first_data_sector;
        let cluster_count = data_sectors / sectors_per_cluster as u32;
        if cluster_count < 65_525 {
            return Err("volume does not contain enough clusters to be FAT32");
        }

        Ok(Self {
            device,
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sectors,
            fat_count,
            fat_size_sectors,
            root_cluster,
            first_data_sector,
        })
    }

    #[must_use]
    pub const fn bytes_per_cluster(&self) -> usize {
        self.bytes_per_sector as usize * self.sectors_per_cluster as usize
    }

    pub fn lookup(&self, path: &[u8]) -> Result<DirectoryEntry, &'static str> {
        let mut cluster = self.root_cluster;
        let mut cursor = PathCursor::new(path);
        let mut found = None;
        while let Some(component) = cursor.next_component()? {
            let entry = self.find_in_directory(cluster, &component)?
                .ok_or("FAT32 path component was not found")?;
            found = Some(entry);
            if cursor.has_more() {
                if !entry.is_directory() {
                    return Err("FAT32 path crosses a non-directory entry");
                }
                cluster = entry.first_cluster;
            }
        }
        found.ok_or("FAT32 path contains no component")
    }

    pub fn read_file(&self, path: &[u8], output: &mut [u8]) -> Result<usize, &'static str> {
        let entry = self.lookup(path)?;
        if entry.is_directory() {
            return Err("FAT32 path names a directory");
        }
        let file_size = entry.byte_size as usize;
        if file_size > output.len() {
            return Err("file scratch buffer is too small");
        }
        if file_size == 0 {
            return Ok(0);
        }

        let mut cluster = entry.first_cluster;
        let mut written = 0usize;
        while cluster >= 2 && cluster < FAT32_EOC && written < file_size {
            let first_sector = self.cluster_first_sector(cluster)?;
            for sector_index in 0..self.sectors_per_cluster as u32 {
                if written == file_size {
                    break;
                }
                let mut sector = [0u8; SECTOR_SIZE];
                self.device.read_sector((first_sector + sector_index) as u64, &mut sector)?;
                let count = core::cmp::min(SECTOR_SIZE, file_size - written);
                output[written..written + count].copy_from_slice(&sector[..count]);
                written += count;
            }
            if written < file_size {
                cluster = self.next_cluster(cluster)?;
            }
        }
        if written != file_size {
            return Err("FAT32 cluster chain ended before the file size");
        }
        Ok(written)
    }

    pub fn visit_directory(
        &self,
        path: &[u8],
        mut visitor: impl FnMut(DirectoryEntry),
    ) -> Result<(), &'static str> {
        let cluster = if path.is_empty() || path == b"C:\\" || path == b"\\" {
            self.root_cluster
        } else {
            let entry = self.lookup(path)?;
            if !entry.is_directory() {
                return Err("FAT32 listing target is not a directory");
            }
            entry.first_cluster
        };
        self.visit_directory_cluster(cluster, &mut visitor)
    }

    fn find_in_directory(
        &self,
        cluster: u32,
        target: &[u8; 11],
    ) -> Result<Option<DirectoryEntry>, &'static str> {
        let mut result = None;
        self.visit_directory_cluster(cluster, &mut |entry| {
            if result.is_none() && &entry.short_name == target {
                result = Some(entry);
            }
        })?;
        Ok(result)
    }

    fn visit_directory_cluster(
        &self,
        mut cluster: u32,
        visitor: &mut impl FnMut(DirectoryEntry),
    ) -> Result<(), &'static str> {
        let mut chain_guard = 0usize;
        loop {
            chain_guard += 1;
            if chain_guard > 4096 {
                return Err("FAT32 directory cluster chain is cyclic");
            }
            let first_sector = self.cluster_first_sector(cluster)?;
            for sector_index in 0..self.sectors_per_cluster as u32 {
                let mut sector = [0u8; SECTOR_SIZE];
                self.device.read_sector((first_sector + sector_index) as u64, &mut sector)?;
                for slot in 0..(SECTOR_SIZE / 32) {
                    let offset = slot * 32;
                    let first = sector[offset];
                    if first == 0x00 {
                        return Ok(());
                    }
                    if first == 0xe5 {
                        continue;
                    }
                    let attributes = sector[offset + 11];
                    if attributes == ATTR_LONG_NAME || attributes & ATTR_VOLUME_ID != 0 {
                        continue;
                    }
                    let mut short_name = [0u8; 11];
                    short_name.copy_from_slice(&sector[offset..offset + 11]);
                    let high = read_u16(&sector, offset + 20)? as u32;
                    let low = read_u16(&sector, offset + 26)? as u32;
                    visitor(DirectoryEntry {
                        short_name,
                        attributes,
                        first_cluster: (high << 16) | low,
                        byte_size: read_u32(&sector, offset + 28)?,
                    });
                }
            }
            cluster = self.next_cluster(cluster)?;
            if cluster >= FAT32_EOC {
                return Ok(());
            }
        }
    }

    fn cluster_first_sector(&self, cluster: u32) -> Result<u32, &'static str> {
        if cluster < 2 {
            return Err("FAT32 cluster number is reserved");
        }
        self.first_data_sector
            .checked_add((cluster - 2).saturating_mul(self.sectors_per_cluster as u32))
            .ok_or("FAT32 cluster-sector calculation overflow")
    }

    fn next_cluster(&self, cluster: u32) -> Result<u32, &'static str> {
        let fat_offset = cluster.checked_mul(4).ok_or("FAT32 FAT offset overflow")?;
        let fat_sector = self.reserved_sectors as u32 + fat_offset / SECTOR_SIZE as u32;
        let entry_offset = (fat_offset % SECTOR_SIZE as u32) as usize;
        if fat_sector >= self.reserved_sectors as u32 + self.fat_size_sectors {
            return Err("FAT32 cluster points outside FAT");
        }
        let mut sector = [0u8; SECTOR_SIZE];
        self.device.read_sector(fat_sector as u64, &mut sector)?;
        Ok(read_u32(&sector, entry_offset)? & 0x0fff_ffff)
    }
}

struct PathCursor<'a> {
    path: &'a [u8],
    offset: usize,
}

impl<'a> PathCursor<'a> {
    const fn new(path: &'a [u8]) -> Self {
        Self { path, offset: 0 }
    }

    fn has_more(&self) -> bool {
        self.path[self.offset..]
            .iter()
            .any(|byte| !matches!(*byte, b'\\' | b'/' | b':' | b' '))
    }

    fn next_component(&mut self) -> Result<Option<[u8; 11]>, &'static str> {
        while self.offset < self.path.len()
            && matches!(self.path[self.offset], b'\\' | b'/' | b':' | b' ')
        {
            self.offset += 1;
        }
        if self.offset >= self.path.len() {
            return Ok(None);
        }
        if self.offset == 0
            && self.path.len() >= 2
            && self.path[1] == b':'
        {
            self.offset = 2;
            return self.next_component();
        }

        let start = self.offset;
        while self.offset < self.path.len() && !matches!(self.path[self.offset], b'\\' | b'/') {
            self.offset += 1;
        }
        let component = &self.path[start..self.offset];
        if component.is_empty() {
            return Ok(None);
        }
        short_name(component).map(Some)
    }
}

fn short_name(component: &[u8]) -> Result<[u8; 11], &'static str> {
    let mut output = [b' '; 11];
    let mut name_index = 0usize;
    let mut extension_index = 8usize;
    let mut in_extension = false;
    for &byte in component {
        if byte == b'.' {
            if in_extension {
                return Err("FAT32 short name contains multiple dots");
            }
            in_extension = true;
            continue;
        }
        let upper = byte.to_ascii_uppercase();
        if !(upper.is_ascii_alphanumeric() || matches!(upper, b'_' | b'-' | b'$' | b'~')) {
            return Err("FAT32 short name contains an unsupported character");
        }
        if in_extension {
            if extension_index >= 11 {
                return Err("FAT32 extension exceeds three characters");
            }
            output[extension_index] = upper;
            extension_index += 1;
        } else {
            if name_index >= 8 {
                return Err("FAT32 base name exceeds eight characters");
            }
            output[name_index] = upper;
            name_index += 1;
        }
    }
    if name_index == 0 {
        return Err("FAT32 short name is empty");
    }
    Ok(output)
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, &'static str> {
    let data = bytes.get(offset..offset + 2).ok_or("FAT32 u16 read exceeds sector")?;
    Ok(u16::from_le_bytes([data[0], data[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, &'static str> {
    let data = bytes.get(offset..offset + 4).ok_or("FAT32 u32 read exceeds sector")?;
    Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}
