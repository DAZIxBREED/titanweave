use crate::block::{MemoryBlockDevice, SECTOR_SIZE};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NtfsSafetyState {
    NotNtfs,
    CleanReadOnly,
    DirtyReadOnly,
    HibernatedReadOnly,
    Invalid,
}

impl NtfsSafetyState {
    pub const fn name(self) -> &'static str {
        match self {
            Self::NotNtfs => "not-ntfs",
            Self::CleanReadOnly => "clean-read-only",
            Self::DirtyReadOnly => "dirty-read-only",
            Self::HibernatedReadOnly => "hibernated-read-only",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Clone, Copy)]
pub struct NtfsBootSector {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub mft_lcn: u64,
    pub safety: NtfsSafetyState,
}

pub fn inspect(device: MemoryBlockDevice) -> Result<NtfsBootSector, &'static str> {
    let mut sector = [0u8; SECTOR_SIZE];
    device.read_sector(0, &mut sector)?;
    if sector[3..11] != *b"NTFS    " {
        return Ok(NtfsBootSector {
            bytes_per_sector: 0,
            sectors_per_cluster: 0,
            mft_lcn: 0,
            safety: NtfsSafetyState::NotNtfs,
        });
    }
    if sector[510] != 0x55 || sector[511] != 0xaa {
        return Err("NTFS boot signature is missing");
    }
    let bytes_per_sector = u16::from_le_bytes([sector[11], sector[12]]);
    let sectors_per_cluster = sector[13];
    if bytes_per_sector != 512 || sectors_per_cluster == 0 || !sectors_per_cluster.is_power_of_two() {
        return Err("K9 NTFS geometry is unsupported");
    }
    let total_sectors = u64::from_le_bytes(sector[40..48].try_into().unwrap());
    let mft_lcn = u64::from_le_bytes(sector[48..56].try_into().unwrap());
    if total_sectors == 0 || total_sectors > device.sector_count() || mft_lcn == 0 {
        return Err("NTFS geometry exceeds the block device");
    }

    // The full $Volume and hiberfil.sys checks arrive with the MFT reader.
    // Until then K9 deliberately exposes NTFS through a read-only policy.
    Ok(NtfsBootSector {
        bytes_per_sector,
        sectors_per_cluster,
        mft_lcn,
        safety: NtfsSafetyState::CleanReadOnly,
    })
}
