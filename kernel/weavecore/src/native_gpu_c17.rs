//! K14.C17 AMD IP Discovery parser and Navi48 exact-base resolution gate.
//!
//! C17 imports the public AMD IP Discovery binary layout used by amdgpu and
//! implements a bounded, little-endian parser for discovery snapshots.  It does
//! not yet fetch the table from physical Radeon VRAM; that access is a separate
//! hardware step.  Therefore Navi48 GC base resolution remains fail-closed until
//! a live, checksum-verified snapshot is supplied.  No MMIO write is promoted.

use crate::{native_gpu_c9,native_gpu_c16,serial,sync::SpinLock};

pub const K14C17_ABI_VERSION:u32=1;
pub const AMD_DISCOVERY_BINARY_SIGNATURE:u32=0x2821_1407;
pub const AMD_DISCOVERY_TABLE_SIGNATURE:u32=0x5344_5049;
pub const AMD_DISCOVERY_MAX_BINARY_BYTES:usize=64*1024;
pub const AMD_DISCOVERY_MAX_TABLES:u16=64;
pub const AMD_DISCOVERY_MAX_DIES:u16=16;
pub const AMD_DISCOVERY_MAX_BASES:u8=16;
pub const RADEON_C17_LIVE_VRAM_DISCOVERY_READ_ALLOWED:bool=false;
pub const RADEON_C17_MMIO_WRITE_ALLOWED:bool=false;
pub const RADEON_C17_FIRMWARE_UPLOAD_ALLOWED:bool=false;
pub const RADEON_C17_COMMAND_SUBMIT_ALLOWED:bool=false;
pub const RADEON_C17_BUS_MASTER_ALLOWED:bool=false;

#[derive(Clone,Copy,Debug)]
pub struct DiscoveryParse { pub valid:bool,pub version_major:u16,pub version_minor:u16,pub binary_size:u16,pub ip_table_offset:u16,pub ip_table_size:u16,pub ip_version:u16,pub num_dies:u16,pub base_addr_64_bit:bool }
impl DiscoveryParse { pub const EMPTY:Self=Self{valid:false,version_major:0,version_minor:0,binary_size:0,ip_table_offset:0,ip_table_size:0,ip_version:0,num_dies:0,base_addr_64_bit:false}; }

#[derive(Clone,Copy,Debug)]
pub struct C17State { pub amd_present:bool,pub navi48:bool,pub c16_ready:bool,pub parser_ready:bool,pub source_layout_imported:bool,pub synthetic_selftest_passed:bool,pub live_snapshot_available:bool,pub live_snapshot_verified:bool,pub exact_gc_base_resolved:bool,pub c16_target_promotable:bool,pub mmio_write_enabled:bool,pub firmware_upload_enabled:bool,pub command_submit_enabled:bool,pub radeon_bus_master_enabled:bool,pub fallback_armed:bool,pub device_id:u16,pub revision:u8 }
impl C17State { pub const EMPTY:Self=Self{amd_present:false,navi48:false,c16_ready:false,parser_ready:false,source_layout_imported:false,synthetic_selftest_passed:false,live_snapshot_available:false,live_snapshot_verified:false,exact_gc_base_resolved:false,c16_target_promotable:false,mmio_write_enabled:false,firmware_upload_enabled:false,command_submit_enabled:false,radeon_bus_master_enabled:false,fallback_armed:true,device_id:0,revision:0}; }
static STATE:SpinLock<C17State>=SpinLock::new(C17State::EMPTY);

fn le16(b:&[u8],o:usize)->Option<u16>{Some(u16::from_le_bytes([*b.get(o)?,*b.get(o+1)?]))}
fn le32(b:&[u8],o:usize)->Option<u32>{Some(u32::from_le_bytes([*b.get(o)?,*b.get(o+1)?,*b.get(o+2)?,*b.get(o+3)?]))}

/// Parse only the AMD binary header and IP-discovery table header.  This is
/// intentionally bounded and does not trust offsets/sizes until checked.
pub fn parse_discovery_snapshot(b:&[u8])->Result<DiscoveryParse,&'static str>{
 if b.len()<60 || b.len()>AMD_DISCOVERY_MAX_BINARY_BYTES{return Err("K14.C17 discovery snapshot size invalid");}
 if le32(b,0)!=Some(AMD_DISCOVERY_BINARY_SIGNATURE){return Err("K14.C17 AMD discovery binary signature mismatch");}
 let maj=le16(b,4).ok_or("K14.C17 truncated version")?;let min=le16(b,6).ok_or("K14.C17 truncated version")?;let size=le16(b,10).ok_or("K14.C17 truncated binary size")?;
 if size as usize>b.len()||size<20{return Err("K14.C17 binary size out of bounds");}
 // v1 header has six fixed table_info records starting at byte 12. v2 adds
 // num_tables/padding and then a variable table list at byte 16.
 let (table_count,table_base)=if maj>=2{let n=le16(b,12).ok_or("K14.C17 truncated table count")?;if n==0||n>AMD_DISCOVERY_MAX_TABLES{return Err("K14.C17 invalid table count");}(n,16usize)}else{(6u16,12usize)};
 if table_base+table_count as usize*8>size as usize{return Err("K14.C17 table list exceeds binary");}
 let ip_off=le16(b,table_base).ok_or("K14.C17 missing IP table offset")?;let ip_size=le16(b,table_base+4).ok_or("K14.C17 missing IP table size")?;
 let end=ip_off as usize+ip_size as usize;if ip_size<44||end>size as usize{return Err("K14.C17 IP discovery table bounds invalid");}
 if le32(b,ip_off as usize)!=Some(AMD_DISCOVERY_TABLE_SIGNATURE){return Err("K14.C17 IP discovery signature mismatch");}
 let ip_ver=le16(b,ip_off as usize+4).ok_or("K14.C17 truncated IP version")?;let dies=le16(b,ip_off as usize+12).ok_or("K14.C17 truncated die count")?;
 if dies==0||dies>AMD_DISCOVERY_MAX_DIES{return Err("K14.C17 invalid die count");}
 let flags=if ip_ver==4{*b.get(ip_off as usize+78).ok_or("K14.C17 truncated v4 flags")?}else{0};
 Ok(DiscoveryParse{valid:true,version_major:maj,version_minor:min,binary_size:size,ip_table_offset:ip_off,ip_table_size:ip_size,ip_version:ip_ver,num_dies:dies,base_addr_64_bit:flags&1!=0})
}

fn parser_self_test()->Result<(),&'static str>{
 let mut b=[0u8;128];b[0..4].copy_from_slice(&AMD_DISCOVERY_BINARY_SIGNATURE.to_le_bytes());b[4..6].copy_from_slice(&1u16.to_le_bytes());b[10..12].copy_from_slice(&128u16.to_le_bytes());
 // table_list[IP_DISCOVERY] at 12: offset=64, size=64
 b[12..14].copy_from_slice(&64u16.to_le_bytes());b[16..18].copy_from_slice(&64u16.to_le_bytes());
 b[64..68].copy_from_slice(&AMD_DISCOVERY_TABLE_SIGNATURE.to_le_bytes());b[68..70].copy_from_slice(&3u16.to_le_bytes());b[70..72].copy_from_slice(&64u16.to_le_bytes());b[76..78].copy_from_slice(&1u16.to_le_bytes());
 let p=parse_discovery_snapshot(&b)?;if !p.valid||p.ip_table_offset!=64||p.ip_version!=3||p.num_dies!=1{return Err("K14.C17 parser self-test mismatch");}Ok(())
}

pub fn initialize()->Result<C17State,&'static str>{
 if K14C17_ABI_VERSION!=1||RADEON_C17_LIVE_VRAM_DISCOVERY_READ_ALLOWED||RADEON_C17_MMIO_WRITE_ALLOWED||RADEON_C17_FIRMWARE_UPLOAD_ALLOWED||RADEON_C17_COMMAND_SUBMIT_ALLOWED||RADEON_C17_BUS_MASTER_ALLOWED{return Err("K14.C17 fail-closed constants invalid");}
 parser_self_test()?;let c9=native_gpu_c9::state();let c16=native_gpu_c16::state();let mut s=C17State{amd_present:c9.amd_present,navi48:c9.profile==native_gpu_c9::ProfileId::Navi48Rx9070,c16_ready:!c9.amd_present||c16.target_reviewed,parser_ready:true,source_layout_imported:true,synthetic_selftest_passed:true,device_id:c9.device_id,revision:c9.revision,..C17State::EMPTY};
 serial::println(format_args!("[C17IP] AMD IP-discovery parser: binary_sig={:#010x} table_sig={:#010x} bounded=true endian=little max_binary={} max_tables={} max_dies={} source_layout_imported=true",AMD_DISCOVERY_BINARY_SIGNATURE,AMD_DISCOVERY_TABLE_SIGNATURE,AMD_DISCOVERY_MAX_BINARY_BYTES,AMD_DISCOVERY_MAX_TABLES,AMD_DISCOVERY_MAX_DIES));
 serial::println(format_args!("[C17PG] Navi48 resolution policy: require=verified_live_discovery_snapshot+exact_GC_ip_entry+exact_base_address+C16_reviewed_target; live_vram_read=false guessed_bases=false MMIO_write=false firmware=false submit=false bus_master_enable=false"));
 if !s.amd_present{serial::println(format_args!("[C17HW] Navi48 IP discovery: present=false qemu_deferred=true parser_ready=true snapshot=false gc_base_resolved=false fallback=true"));}
 else if s.navi48{serial::println(format_args!("[C17HW] Navi48 IP discovery: present=true devid={:#06x} parser_ready=true snapshot=false gc_base_resolved=false reason=live_discovery_fetch_not_promoted fallback=true",s.device_id));}
 else{serial::println(format_args!("[C17HW] Radeon IP discovery: present=true devid={:#06x} navi48=false parser_ready=true snapshot=false gc_base_resolved=false fallback=true",s.device_id));}
 if s.live_snapshot_verified&&!s.live_snapshot_available{return Err("K14.C17 verified snapshot without snapshot");}
 if s.exact_gc_base_resolved&&!s.live_snapshot_verified{return Err("K14.C17 exact GC base resolved without verified discovery");}
 if s.c16_target_promotable&&!s.exact_gc_base_resolved{return Err("K14.C17 C16 target promoted without exact GC base");}
 if s.mmio_write_enabled||s.firmware_upload_enabled||s.command_submit_enabled||s.radeon_bus_master_enabled{return Err("K14.C17 destructive capability promoted early");}
 serial::println(format_args!("[C17RD] K14.C17 IP-discovery gate ready: amd_present={} navi48={} C16_ready={} parser={} source_layout={} selftest={} snapshot={} verified={} gc_base={} promotable={} fallback=true",s.amd_present,s.navi48,s.c16_ready,s.parser_ready,s.source_layout_imported,s.synthetic_selftest_passed,s.live_snapshot_available,s.live_snapshot_verified,s.exact_gc_base_resolved,s.c16_target_promotable));
 *STATE.lock()=s;Ok(s)
}
pub fn state()->C17State{*STATE.lock()}
pub fn packed_status()->u64{let s=state();let mut v=(u64::from(s.device_id)<<32)|(u64::from(s.revision)<<24);for(bit,on)in[s.amd_present,s.navi48,s.c16_ready,s.parser_ready,s.source_layout_imported,s.synthetic_selftest_passed,s.live_snapshot_available,s.live_snapshot_verified,s.exact_gc_base_resolved,s.c16_target_promotable,s.fallback_armed].into_iter().enumerate(){if on{v|=1u64<<bit;}}v}
