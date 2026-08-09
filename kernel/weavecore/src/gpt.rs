use crate::block::{MemoryBlockDevice, SECTOR_SIZE};

const GPT_SIGNATURE: &[u8; 8] = b"EFI PART";
const MAX_PARTITION_ENTRIES: u32 = 4096;
pub const MAX_DISCOVERED_PARTITIONS: usize = 32;

#[derive(Clone, Copy)]
pub struct GptPartition {
    pub type_guid: [u8; 16],
    pub unique_guid: [u8; 16],
    pub first_lba: u64,
    pub last_lba: u64,
    pub attributes: u64,
    pub name_utf16: [u16; 36],
}

impl GptPartition {
    pub const EMPTY: Self = Self {
        type_guid: [0; 16], unique_guid: [0; 16], first_lba: 0, last_lba: 0,
        attributes: 0, name_utf16: [0; 36],
    };
    pub const fn sector_count(self) -> u64 { self.last_lba - self.first_lba + 1 }
    pub fn ascii_name(&self, output: &mut [u8; 36]) -> usize {
        let mut count = 0;
        while count < self.name_utf16.len() && self.name_utf16[count] != 0 {
            let ch = self.name_utf16[count];
            output[count] = if ch <= 0x7f { ch as u8 } else { b'?' };
            count += 1;
        }
        count
    }
}

#[derive(Clone, Copy)]
pub struct GptReport {
    pub disk_guid: [u8; 16],
    pub first_usable_lba: u64,
    pub last_usable_lba: u64,
    pub partition_count: usize,
    pub discovered_count: usize,
    pub partitions: [GptPartition; MAX_DISCOVERED_PARTITIONS],
}

pub fn inspect(device: MemoryBlockDevice) -> Result<GptReport, &'static str> {
    if device.sector_count() < 3 { return Err("device is too small for GPT"); }
    let mut header = [0u8; SECTOR_SIZE];
    device.read_sector(1, &mut header)?;
    if &header[0..8] != GPT_SIGNATURE { return Err("GPT signature is missing"); }
    let header_size = read_u32(&header, 12)?;
    if !(92..=SECTOR_SIZE as u32).contains(&header_size) { return Err("GPT header size is invalid"); }
    if read_u64(&header, 24)? != 1 { return Err("primary GPT header is not at LBA 1"); }
    let first_usable_lba = read_u64(&header, 40)?;
    let last_usable_lba = read_u64(&header, 48)?;
    if first_usable_lba > last_usable_lba || last_usable_lba >= device.sector_count() {
        return Err("GPT usable range exceeds the device");
    }
    let entries_lba = read_u64(&header, 72)?;
    let entry_count = read_u32(&header, 80)?;
    let entry_size = read_u32(&header, 84)?;
    if entry_count == 0 || entry_count > MAX_PARTITION_ENTRIES { return Err("GPT entry count is invalid"); }
    if entry_size < 128 || entry_size > 1024 || entry_size % 8 != 0 { return Err("GPT entry size is invalid"); }
    let table_bytes = (entry_count as u64).checked_mul(entry_size as u64).ok_or("GPT table size overflow")?;
    let table_sectors = table_bytes.div_ceil(SECTOR_SIZE as u64);
    if entries_lba == 0 || entries_lba.saturating_add(table_sectors) > device.sector_count() {
        return Err("GPT partition table exceeds the device");
    }
    let mut disk_guid = [0u8; 16]; disk_guid.copy_from_slice(&header[56..72]);
    let (partition_count, discovered_count, partitions) = read_entries(device, entries_lba, entry_count, entry_size, first_usable_lba, last_usable_lba)?;
    Ok(GptReport { disk_guid, first_usable_lba, last_usable_lba, partition_count, discovered_count, partitions })
}

fn read_entries(device: MemoryBlockDevice, table_lba: u64, entry_count: u32, entry_size: u32,
    first_usable: u64, last_usable: u64) -> Result<(usize, usize, [GptPartition; MAX_DISCOVERED_PARTITIONS]), &'static str> {
    let mut partitions = [GptPartition::EMPTY; MAX_DISCOVERED_PARTITIONS];
    let mut used = 0usize; let mut stored = 0usize;
    let mut entry = [0u8; 1024];
    for index in 0..entry_count as u64 {
        read_entry_bytes(device, table_lba, index * entry_size as u64, entry_size as usize, &mut entry)?;
        if entry[..16].iter().all(|b| *b == 0) { continue; }
        used += 1;
        if stored == MAX_DISCOVERED_PARTITIONS { continue; }
        let first_lba = u64::from_le_bytes(entry[32..40].try_into().unwrap());
        let last_lba = u64::from_le_bytes(entry[40..48].try_into().unwrap());
        if first_lba > last_lba || first_lba < first_usable || last_lba > last_usable {
            return Err("GPT partition range is invalid");
        }
        let mut part = GptPartition::EMPTY;
        part.type_guid.copy_from_slice(&entry[0..16]);
        part.unique_guid.copy_from_slice(&entry[16..32]);
        part.first_lba = first_lba; part.last_lba = last_lba;
        part.attributes = u64::from_le_bytes(entry[48..56].try_into().unwrap());
        for n in 0..36 { let o=56+n*2; part.name_utf16[n]=u16::from_le_bytes([entry[o],entry[o+1]]); }
        partitions[stored] = part; stored += 1;
    }
    Ok((used, stored, partitions))
}

fn read_entry_bytes(device: MemoryBlockDevice, table_lba: u64, byte_offset: u64, len: usize, output: &mut [u8;1024]) -> Result<(), &'static str> {
    let mut copied=0usize;
    while copied < len {
        let absolute = byte_offset + copied as u64;
        let lba = table_lba + absolute / SECTOR_SIZE as u64;
        let offset = (absolute % SECTOR_SIZE as u64) as usize;
        let mut sector=[0u8;SECTOR_SIZE]; device.read_sector(lba,&mut sector)?;
        let take=core::cmp::min(len-copied, SECTOR_SIZE-offset);
        output[copied..copied+take].copy_from_slice(&sector[offset..offset+take]); copied+=take;
    }
    Ok(())
}
fn read_u32(bytes:&[u8], offset:usize)->Result<u32,&'static str>{let v=bytes.get(offset..offset+4).ok_or("GPT u32 outside sector")?;Ok(u32::from_le_bytes(v.try_into().unwrap()))}
fn read_u64(bytes:&[u8], offset:usize)->Result<u64,&'static str>{let v=bytes.get(offset..offset+8).ok_or("GPT u64 outside sector")?;Ok(u64::from_le_bytes(v.try_into().unwrap()))}
