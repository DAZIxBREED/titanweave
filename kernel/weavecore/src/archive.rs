//! K9 Vaultforge archive format registry and hardened archive inspection.
//!
//! This module intentionally separates format recognition from codec execution.
//! The kernel validates containers and policy; the user-space Titan Archive
//! Service owns heavy compression/decompression engines.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveCapability {
    PackAndUnpack,
    UnpackOnly,
    InspectOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveFormat {
    SevenZip,
    Xz,
    Bzip2,
    Gzip,
    Tar,
    Zip,
    Wim,
    Apfs,
    Ar,
    Arj,
    Cab,
    Chm,
    Cpio,
    CramFs,
    Dmg,
    Ext,
    Fat,
    Gpt,
    Hfs,
    Ihex,
    Iso,
    Lzh,
    Lzma,
    Mbr,
    Msi,
    Nsis,
    Ntfs,
    Qcow2,
    Rar,
    Rpm,
    SquashFs,
    Udf,
    Uefi,
    Vdi,
    Vhd,
    Vhdx,
    Vmdk,
    Xar,
    Z,
    Unknown,
}

impl ArchiveFormat {
    pub const fn capability(self) -> ArchiveCapability {
        match self {
            Self::SevenZip | Self::Xz | Self::Bzip2 | Self::Gzip | Self::Tar |
            Self::Zip | Self::Wim => ArchiveCapability::PackAndUnpack,
            Self::Unknown => ArchiveCapability::InspectOnly,
            _ => ArchiveCapability::UnpackOnly,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::SevenZip => "7z", Self::Xz => "xz", Self::Bzip2 => "bzip2",
            Self::Gzip => "gzip", Self::Tar => "tar", Self::Zip => "zip",
            Self::Wim => "wim", Self::Apfs => "apfs", Self::Ar => "ar",
            Self::Arj => "arj", Self::Cab => "cab", Self::Chm => "chm",
            Self::Cpio => "cpio", Self::CramFs => "cramfs", Self::Dmg => "dmg",
            Self::Ext => "ext", Self::Fat => "fat", Self::Gpt => "gpt",
            Self::Hfs => "hfs", Self::Ihex => "ihex", Self::Iso => "iso",
            Self::Lzh => "lzh", Self::Lzma => "lzma", Self::Mbr => "mbr",
            Self::Msi => "msi", Self::Nsis => "nsis", Self::Ntfs => "ntfs",
            Self::Qcow2 => "qcow2", Self::Rar => "rar", Self::Rpm => "rpm",
            Self::SquashFs => "squashfs", Self::Udf => "udf", Self::Uefi => "uefi",
            Self::Vdi => "vdi", Self::Vhd => "vhd", Self::Vhdx => "vhdx",
            Self::Vmdk => "vmdk", Self::Xar => "xar", Self::Z => "z",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ArchiveProbe {
    pub format: ArchiveFormat,
    pub capability: ArchiveCapability,
    pub encrypted: bool,
    pub header_valid: bool,
}

pub fn probe(bytes: &[u8]) -> ArchiveProbe {
    let format = if bytes.starts_with(&[0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c]) {
        ArchiveFormat::SevenZip
    } else if bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06") {
        ArchiveFormat::Zip
    } else if bytes.starts_with(&[0x1f, 0x8b]) {
        ArchiveFormat::Gzip
    } else if bytes.starts_with(b"BZh") {
        ArchiveFormat::Bzip2
    } else if bytes.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0x00]) {
        ArchiveFormat::Xz
    } else if bytes.len() > 262 && &bytes[257..262] == b"ustar" {
        ArchiveFormat::Tar
    } else if bytes.starts_with(b"MSWIM\0\0\0") {
        ArchiveFormat::Wim
    } else if bytes.starts_with(b"Rar!\x1a\x07") {
        ArchiveFormat::Rar
    } else if bytes.starts_with(b"MSCF") {
        ArchiveFormat::Cab
    } else if bytes.starts_with(b"MZ") {
        ArchiveFormat::Msi
    } else {
        ArchiveFormat::Unknown
    };
    ArchiveProbe {
        capability: format.capability(),
        format,
        encrypted: false,
        header_valid: format != ArchiveFormat::Unknown,
    }
}

pub const MAX_ARCHIVE_FILES: u64 = 1_000_000;
pub const MAX_DIRECTORY_DEPTH: u16 = 256;
pub const MAX_EXPANSION_RATIO: u64 = 10_000;

pub fn validate_relative_path(path: &[u8]) -> Result<(), &'static str> {
    if path.is_empty() { return Err("archive entry path is empty"); }
    if path.iter().any(|byte| *byte == 0) { return Err("archive entry contains NUL"); }
    if path[0] == b'/' || path[0] == b'\\' { return Err("absolute archive path denied"); }
    if path.len() >= 2 && path[1] == b':' { return Err("drive-qualified archive path denied"); }
    let mut component_start = 0usize;
    for index in 0..=path.len() {
        if index == path.len() || path[index] == b'/' || path[index] == b'\\' {
            let part = &path[component_start..index];
            if part.is_empty() { return Err("empty archive path component denied"); }
            if part == b"." || part == b".." { return Err("archive path traversal denied"); }
            component_start = index.saturating_add(1);
        }
    }
    Ok(())
}

pub fn validate_expansion(compressed: u64, expanded: u64, files: u64) -> Result<(), &'static str> {
    if files > MAX_ARCHIVE_FILES { return Err("archive file-count limit exceeded"); }
    if compressed == 0 && expanded != 0 { return Err("invalid zero-sized compressed stream"); }
    if compressed != 0 {
        let maximum = compressed.checked_mul(MAX_EXPANSION_RATIO)
            .ok_or("archive expansion limit overflow")?;
        if expanded > maximum { return Err("archive expansion-ratio limit exceeded"); }
    }
    Ok(())
}
