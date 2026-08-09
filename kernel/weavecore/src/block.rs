use core::ptr;
pub const SECTOR_SIZE: usize = 512;

/// Common synchronous block-device contract. Hardware drivers may complete
/// requests asynchronously internally, but filesystems never depend on a
/// one-off proof function.
pub trait BlockDevice {
    fn sector_count(&self) -> u64;
    fn read_sector(&mut self, sector: u64, output: &mut [u8; SECTOR_SIZE]) -> Result<(), &'static str>;
    fn write_sector(&mut self, sector: u64, input: &[u8; SECTOR_SIZE]) -> Result<(), &'static str>;
    fn flush(&mut self) -> Result<(), &'static str>;
    fn read_sectors(&mut self, first: u64, output: &mut [u8]) -> Result<(), &'static str> {
        if output.is_empty() || output.len() % SECTOR_SIZE != 0 { return Err("block read buffer is not sector aligned"); }
        for (offset, chunk) in output.chunks_exact_mut(SECTOR_SIZE).enumerate() {
            let mut sector = [0u8; SECTOR_SIZE];
            self.read_sector(first.checked_add(offset as u64).ok_or("block LBA overflow")?, &mut sector)?;
            chunk.copy_from_slice(&sector);
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct MemoryBlockDevice { base: u64, byte_len: usize, writable: bool }
impl MemoryBlockDevice {
    pub fn new(base: u64, byte_len: usize) -> Result<Self, &'static str> { Self::with_write_access(base, byte_len, false) }
    pub fn with_write_access(base: u64, byte_len: usize, writable: bool) -> Result<Self, &'static str> {
        if base == 0 || byte_len < SECTOR_SIZE || byte_len % SECTOR_SIZE != 0 { return Err("bootstrap block image is not sector aligned"); }
        Ok(Self { base, byte_len, writable })
    }
    fn offset(&self, sector: u64) -> Result<usize, &'static str> {
        let offset = usize::try_from(sector).ok().and_then(|value| value.checked_mul(SECTOR_SIZE)).ok_or("block-sector offset overflow")?;
        if offset.checked_add(SECTOR_SIZE).ok_or("block-sector end overflow")? > self.byte_len { return Err("block access exceeds image"); }
        Ok(offset)
    }
    #[must_use] pub const fn sector_count(&self) -> u64 { (self.byte_len / SECTOR_SIZE) as u64 }
    pub fn slice(&self, first_sector: u64, sector_count: u64) -> Result<Self, &'static str> {
        if sector_count == 0 {
            return Err("block slice cannot be empty");
        }
        let first_byte = usize::try_from(first_sector)
            .ok()
            .and_then(|value| value.checked_mul(SECTOR_SIZE))
            .ok_or("block slice start overflow")?;
        let byte_len = usize::try_from(sector_count)
            .ok()
            .and_then(|value| value.checked_mul(SECTOR_SIZE))
            .ok_or("block slice length overflow")?;
        let end = first_byte
            .checked_add(byte_len)
            .ok_or("block slice end overflow")?;
        if end > self.byte_len {
            return Err("block slice exceeds image");
        }
        let base = self.base
            .checked_add(first_byte as u64)
            .ok_or("block slice base overflow")?;
        Ok(Self { base, byte_len, writable: self.writable })
    }
    pub fn read_sector(&self, sector: u64, output: &mut [u8; SECTOR_SIZE]) -> Result<(), &'static str> {
        let offset = self.offset(sector)?;
        unsafe { ptr::copy_nonoverlapping((self.base as *const u8).add(offset), output.as_mut_ptr(), SECTOR_SIZE) };
        Ok(())
    }
}
impl BlockDevice for MemoryBlockDevice {
    fn sector_count(&self) -> u64 { self.sector_count() }
    fn read_sector(&mut self, sector: u64, output: &mut [u8; SECTOR_SIZE]) -> Result<(), &'static str> { MemoryBlockDevice::read_sector(self, sector, output) }
    fn write_sector(&mut self, sector: u64, input: &[u8; SECTOR_SIZE]) -> Result<(), &'static str> {
        if !self.writable { return Err("block device is read-only"); }
        let offset = self.offset(sector)?;
        unsafe { ptr::copy_nonoverlapping(input.as_ptr(), (self.base as *mut u8).add(offset), SECTOR_SIZE) };
        Ok(())
    }
    fn flush(&mut self) -> Result<(), &'static str> { Ok(()) }
}
