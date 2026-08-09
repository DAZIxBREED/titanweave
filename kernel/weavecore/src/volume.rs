use crate::block::{MemoryBlockDevice, SECTOR_SIZE};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FilesystemKind { Fat32, Ntfs, Exfat, Iso9660, Udf, Unknown }
impl FilesystemKind { pub const fn name(self)->&'static str { match self {Self::Fat32=>"FAT32",Self::Ntfs=>"NTFS",Self::Exfat=>"exFAT",Self::Iso9660=>"ISO9660",Self::Udf=>"UDF",Self::Unknown=>"unknown"} } }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MountPolicy { ReadWrite, ReadOnly, Hidden, Quarantine, Locked }
impl MountPolicy { pub const fn name(self)->&'static str { match self {Self::ReadWrite=>"read-write",Self::ReadOnly=>"read-only",Self::Hidden=>"hidden",Self::Quarantine=>"quarantine",Self::Locked=>"locked"} } }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TrustClass { System, Personal, Shared, Untrusted, Quarantined }
impl TrustClass { pub const fn name(self)->&'static str { match self {Self::System=>"system",Self::Personal=>"personal",Self::Shared=>"shared",Self::Untrusted=>"untrusted",Self::Quarantined=>"quarantined"} } }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VolumeRole { Efi, Recovery, System, Data, Removable, Unknown }
impl VolumeRole { pub const fn name(self)->&'static str { match self {Self::Efi=>"efi",Self::Recovery=>"recovery",Self::System=>"system",Self::Data=>"data",Self::Removable=>"removable",Self::Unknown=>"unknown"} } }

#[derive(Clone, Copy)]
pub struct VolumeProbe { pub filesystem:FilesystemKind, pub policy:MountPolicy, pub trust:TrustClass, pub role:VolumeRole, pub label:[u8;11] }
impl VolumeProbe { pub fn label_str(&self)->&str { let end=self.label.iter().position(|b|*b==0||*b==b' ').unwrap_or(self.label.len()); core::str::from_utf8(&self.label[..end]).unwrap_or("?") } }

pub fn probe(device:MemoryBlockDevice)->Result<VolumeProbe,&'static str>{ probe_with_role(device,VolumeRole::Unknown,false) }
pub fn probe_with_role(device:MemoryBlockDevice, role:VolumeRole, removable:bool)->Result<VolumeProbe,&'static str>{
    let mut sector=[0u8;SECTOR_SIZE]; device.read_sector(0,&mut sector)?;
    let mut fs=FilesystemKind::Unknown; let mut label=*b"UNKNOWN    ";
    if sector[3..11]==*b"NTFS    " {fs=FilesystemKind::Ntfs;label=*b"NTFS       ";}
    else if sector[3..11]==*b"EXFAT   " {fs=FilesystemKind::Exfat;label=*b"EXFAT      ";}
    else if sector[82..90]==*b"FAT32   "&&sector[510]==0x55&&sector[511]==0xaa {fs=FilesystemKind::Fat32;label.copy_from_slice(&sector[71..82]);}
    else if device.sector_count()>17 { let mut iso=[0u8;SECTOR_SIZE]; device.read_sector(16,&mut iso)?; if iso[1..6]==*b"CD001" {fs=FilesystemKind::Iso9660;label=*b"ISO9660    ";} }
    let effective_role=if role!=VolumeRole::Unknown {role} else if removable {VolumeRole::Removable} else {VolumeRole::Data};
    let (policy,trust)=match (effective_role,fs) {
        (VolumeRole::Efi,_)|(VolumeRole::Recovery,_) => (MountPolicy::Hidden,TrustClass::System),
        (_,FilesystemKind::Unknown)=>(MountPolicy::Quarantine,TrustClass::Quarantined),
        (_,FilesystemKind::Iso9660)|(_,FilesystemKind::Udf)=>(MountPolicy::ReadOnly,TrustClass::Shared),
        (VolumeRole::Removable,_) => (MountPolicy::ReadWrite,TrustClass::Untrusted),
        (_,FilesystemKind::Ntfs)=>(MountPolicy::ReadOnly,TrustClass::Personal),
        _=>(MountPolicy::ReadWrite,TrustClass::Personal),
    };
    Ok(VolumeProbe{filesystem:fs,policy,trust,role:effective_role,label})
}
