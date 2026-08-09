//! K14.C28 AMD firmware discovery, validation, and owned GTT staging.
//!
//! C28 parses the real AMD common firmware header, enforces size/range/CRC32
//! integrity, hashes the complete file, and copies validated firmware into
//! Radeon-owned GTT backing.  This is operational firmware *staging* only;
//! silicon upload is intentionally false until the later execution milestone
//! owns the required PSP/IP programming sequence.

use core::ptr;
use crate::{memory::FrameAllocator,radeon_memory,sha256,sync::SpinLock,vfs};

pub const RADEON_FIRMWARE_ABI_VERSION:u32=1;
pub const AMD_COMMON_HEADER_BYTES:usize=32;
pub const MAX_FIRMWARE_COMPONENTS:usize=8;
pub const MAX_FIRMWARE_BYTES:u64=1024*1024;
pub const RADEON_C28_FIRMWARE_UPLOAD_TO_GPU:bool=false;

#[repr(u8)]
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum FirmwareComponent{Rlc=1,Me=2,Mec=3,Pfp=4,Mes=5,Mes1=6,Imu=7,Toc=8}
impl FirmwareComponent{
 pub const fn short_path(self)->&'static [u8]{match self{
  Self::Rlc=>b"C:\\SYSTEM\\FIRMWARE\\GFXRLC.BIN",Self::Me=>b"C:\\SYSTEM\\FIRMWARE\\GFXME.BIN",
  Self::Mec=>b"C:\\SYSTEM\\FIRMWARE\\GFXMEC.BIN",Self::Pfp=>b"C:\\SYSTEM\\FIRMWARE\\GFXPFP.BIN",
  Self::Mes=>b"C:\\SYSTEM\\FIRMWARE\\GFXMES.BIN",Self::Mes1=>b"C:\\SYSTEM\\FIRMWARE\\GFXMES1.BIN",
  Self::Imu=>b"C:\\SYSTEM\\FIRMWARE\\GFXIMU.BIN",Self::Toc=>b"C:\\SYSTEM\\FIRMWARE\\GFXTOC.BIN"}}
}
const COMPONENTS:[FirmwareComponent;MAX_FIRMWARE_COMPONENTS]=[FirmwareComponent::Rlc,FirmwareComponent::Me,FirmwareComponent::Mec,FirmwareComponent::Pfp,FirmwareComponent::Mes,FirmwareComponent::Mes1,FirmwareComponent::Imu,FirmwareComponent::Toc];

#[derive(Clone,Copy,Debug)]
pub struct CommonFirmwareHeader{
 pub size_bytes:u32,pub header_size_bytes:u32,pub header_version_major:u16,pub header_version_minor:u16,
 pub ip_version_major:u16,pub ip_version_minor:u16,pub ucode_version:u32,pub ucode_size_bytes:u32,
 pub ucode_array_offset_bytes:u32,pub crc32:u32,
}

fn le16(b:&[u8],o:usize)->Result<u16,&'static str>{if o+2>b.len(){return Err("firmware u16 outside image")}Ok(u16::from_le_bytes([b[o],b[o+1]]))}
fn le32(b:&[u8],o:usize)->Result<u32,&'static str>{if o+4>b.len(){return Err("firmware u32 outside image")}Ok(u32::from_le_bytes([b[o],b[o+1],b[o+2],b[o+3]]))}

pub fn parse_common_header(image:&[u8])->Result<CommonFirmwareHeader,&'static str>{
 if image.len()<AMD_COMMON_HEADER_BYTES{return Err("AMD firmware image is smaller than common header")}
 let h=CommonFirmwareHeader{size_bytes:le32(image,0)?,header_size_bytes:le32(image,4)?,header_version_major:le16(image,8)?,header_version_minor:le16(image,10)?,ip_version_major:le16(image,12)?,ip_version_minor:le16(image,14)?,ucode_version:le32(image,16)?,ucode_size_bytes:le32(image,20)?,ucode_array_offset_bytes:le32(image,24)?,crc32:le32(image,28)?};
 if h.size_bytes as usize!=image.len(){return Err("AMD firmware declared size does not equal file size")}
 if h.header_size_bytes<AMD_COMMON_HEADER_BYTES as u32||h.header_size_bytes>h.ucode_array_offset_bytes{return Err("AMD firmware header size is invalid")}
 let start=h.ucode_array_offset_bytes as usize;let end=start.checked_add(h.ucode_size_bytes as usize).ok_or("AMD firmware payload range overflow")?;
 if start<AMD_COMMON_HEADER_BYTES||end>image.len()||h.ucode_size_bytes==0{return Err("AMD firmware payload range is invalid")}
 if crc32_ieee(&image[start..end])!=h.crc32{return Err("AMD firmware payload CRC32 mismatch")}
 Ok(h)
}

/// IEEE CRC-32 used by AMD common firmware headers (reflected polynomial).
pub fn crc32_ieee(data:&[u8])->u32{let mut crc=0xffff_ffffu32;for &byte in data{crc^=u32::from(byte);for _ in 0..8{let mask=0u32.wrapping_sub(crc&1);crc=(crc>>1)^(0xedb8_8320&mask)}}!crc}

#[derive(Clone,Copy,Debug)]
pub struct StagedFirmware{
 pub component:FirmwareComponent,pub object_id:u64,pub bytes:u32,pub ucode_bytes:u32,pub ucode_offset:u32,
 pub ip_major:u16,pub ip_minor:u16,pub ucode_version:u32,pub crc32:u32,pub sha256_prefix:u64,pub active:bool,
}
impl StagedFirmware{pub const EMPTY:Self=Self{component:FirmwareComponent::Rlc,object_id:0,bytes:0,ucode_bytes:0,ucode_offset:0,ip_major:0,ip_minor:0,ucode_version:0,crc32:0,sha256_prefix:0,active:false};}

#[derive(Clone,Copy,Debug)]
pub struct RadeonFirmwareState{
 pub parser_verified:bool,pub crc_verified:bool,pub vfs_scan_complete:bool,pub files_found:u8,pub files_staged:u8,
 pub staging_verified:bool,pub gpu_upload_performed:bool,pub staged_bytes:u64,pub digest_fingerprint:u64,
}
impl RadeonFirmwareState{pub const EMPTY:Self=Self{parser_verified:false,crc_verified:false,vfs_scan_complete:false,files_found:0,files_staged:0,staging_verified:false,gpu_upload_performed:false,staged_bytes:0,digest_fingerprint:0};}
static STAGED:SpinLock<[StagedFirmware;MAX_FIRMWARE_COMPONENTS]>=SpinLock::new([StagedFirmware::EMPTY;MAX_FIRMWARE_COMPONENTS]);
static STATE:SpinLock<RadeonFirmwareState>=SpinLock::new(RadeonFirmwareState::EMPTY);

fn self_test()->Result<(),&'static str>{
 if RADEON_FIRMWARE_ABI_VERSION!=1||RADEON_C28_FIRMWARE_UPLOAD_TO_GPU{return Err("Radeon C28 firmware policy constants invalid")}
 if crc32_ieee(b"123456789")!=0xcbf4_3926{return Err("CRC32 implementation self-test failed")}
 let payload=b"TITANWEAVE-C28-FIRMWARE-PARSER";let offset=64usize;let mut image=[0u8;128];let total=offset+payload.len();
 image[0..4].copy_from_slice(&(total as u32).to_le_bytes());image[4..8].copy_from_slice(&(32u32).to_le_bytes());
 image[8..10].copy_from_slice(&(1u16).to_le_bytes());image[10..12].copy_from_slice(&(0u16).to_le_bytes());
 image[12..14].copy_from_slice(&(12u16).to_le_bytes());image[14..16].copy_from_slice(&(0u16).to_le_bytes());
 image[16..20].copy_from_slice(&(0x1234u32).to_le_bytes());image[20..24].copy_from_slice(&(payload.len() as u32).to_le_bytes());
 image[24..28].copy_from_slice(&(offset as u32).to_le_bytes());image[offset..total].copy_from_slice(payload);
 let crc=crc32_ieee(payload);image[28..32].copy_from_slice(&crc.to_le_bytes());let h=parse_common_header(&image[..total])?;
 if h.ip_version_major!=12||h.ucode_version!=0x1234||h.crc32!=crc{return Err("AMD firmware common-header self-test failed")}
 Ok(())
}
fn sha_prefix(d:[u8;32])->u64{u64::from_be_bytes([d[0],d[1],d[2],d[3],d[4],d[5],d[6],d[7]])}
fn mix(mut h:u64,v:u64)->u64{h^=v;h=h.wrapping_mul(0x100000001b3);h}

pub fn initialize(allocator:&mut FrameAllocator<'_>,owner:u64,require_physical_firmware:bool)->Result<RadeonFirmwareState,&'static str>{
 self_test()?;if owner==0&&require_physical_firmware{return Err("physical Radeon firmware staging lacks owner")}
 let mut state=RadeonFirmwareState{parser_verified:true,crc_verified:true,vfs_scan_complete:true,..RadeonFirmwareState::EMPTY};
 let mut slots=[StagedFirmware::EMPTY;MAX_FIRMWARE_COMPONENTS];
 for (index,component) in COMPONENTS.iter().copied().enumerate(){
  let path=component.short_path();if !vfs::file_exists(path){continue}state.files_found=state.files_found.saturating_add(1);
  let staged=vfs::with_file(path,|image|{
   if image.len() as u64>MAX_FIRMWARE_BYTES{return Err("AMD firmware image exceeds C28 staging limit")}
   let header=parse_common_header(image)?;let object=radeon_memory::allocate_gtt(allocator,owner,image.len() as u64,4096)?;
   unsafe{ptr::copy_nonoverlapping(image.as_ptr(),object.kernel_virtual as *mut u8,image.len())}
   let copied=unsafe{core::slice::from_raw_parts(object.kernel_virtual as *const u8,image.len())};
   if !sha256::constant_time_eq(&sha256::digest(image),&sha256::digest(copied)){let _=radeon_memory::free(allocator,owner,object.id);return Err("Radeon firmware GTT staging readback mismatch")}
   radeon_memory::pin(owner,object.id,true)?;let digest=sha256::digest(image);
   Ok(StagedFirmware{component,object_id:object.id,bytes:image.len() as u32,ucode_bytes:header.ucode_size_bytes,ucode_offset:header.ucode_array_offset_bytes,
    ip_major:header.ip_version_major,ip_minor:header.ip_version_minor,ucode_version:header.ucode_version,crc32:header.crc32,sha256_prefix:sha_prefix(digest),active:true})
  })?;
  state.files_staged=state.files_staged.saturating_add(1);state.staged_bytes=state.staged_bytes.saturating_add(u64::from(staged.bytes));state.digest_fingerprint=mix(state.digest_fingerprint,staged.sha256_prefix);slots[index]=staged;
 }
 if require_physical_firmware&&state.files_staged==0{return Err("physical Radeon found but no validated C28 firmware files are staged")}
 state.staging_verified=!require_physical_firmware||state.files_staged>0;state.gpu_upload_performed=false;*STAGED.lock()=slots;*STATE.lock()=state;Ok(state)
}
pub fn state()->RadeonFirmwareState{*STATE.lock()}
pub fn staged()->[StagedFirmware;MAX_FIRMWARE_COMPONENTS]{*STAGED.lock()}
