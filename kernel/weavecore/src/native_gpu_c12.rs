//! K14.C12 trusted Radeon IP-base sources + first live status-read path.
//!
//! C11 introduced reviewed IP-relative register indices but deliberately left
//! the per-IP base map unresolved. C12 adds two trusted base sources:
//!   * Navi 21: AMD's generated sienna_cichlid_ip_offset.h table.
//!   * Discovery-record path: a bounded selector for GC/SDMA0 base records,
//!     intended for Navi 48 and later ASICs whose bases are supplied by AMD IP
//!     discovery data rather than a Titanweave guess.
//!
//! C12 also corrects the Radeon register aperture selection: upstream amdgpu
//! maps the register MMIO aperture from PCI BAR5 on modern (Bonaire+) devices.
//! Titanweave therefore never reuses C7's generic "first memory BAR" for a
//! real register read. Each requested register page is mapped supervisor-only,
//! NX, uncached, and read-only, then exactly one bounded volatile u32 load is
//! performed. No Radeon register write, firmware upload, command submission,
//! or bus-master enable is permitted in C12.
//!
//! Reviewed source facts used by this milestone:
//! - AMD/Linux sienna_cichlid_ip_offset.h:
//!     GC_BASE__INST0_SEG0    = 0x00001260
//!     SDMA0_BASE__INST0_SEG0 = 0x00001260
//! - AMD/Linux gc_10_3_0_offset.h:
//!     GRBM_STATUS            = 0x0da4, BASE_IDX=0
//!     GRBM_CHIP_REVISION     = 0x0dc1, BASE_IDX=0
//!     SDMA0_STATUS_REG       = 0x0025, BASE_IDX=0
//! - AMD/Linux amdgpu_device.c uses PCI BAR5 as rmmio for modern Radeon.
//! - AMD/Linux discovery.h models per-IP base-address records; C12 keeps the
//!   discovery record selector bounded and fail-closed until a trusted binary
//!   acquisition/parser supplies those records.

use crate::{
    memory::FrameAllocator,
    native_gpu_c6,
    native_gpu_c9,
    native_gpu_c11::{self, RegisterIp},
    native_gpu_binding,
    paging,
    pci,
    serial,
    sync::SpinLock,
};

pub const K14C12_ABI_VERSION: u32 = 1;
pub const RADEON_C12_MMIO_BAR_INDEX: u8 = 5;
pub const RADEON_C12_PAGE_BYTES: u64 = 4096;
pub const RADEON_C12_MAX_LIVE_READS: u8 = 3;
pub const RADEON_C12_MMIO_WRITES_ALLOWED: bool = false;
pub const RADEON_C12_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C12_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C12_BUS_MASTER_ALLOWED: bool = false;

const NAVI21_GC_BASE0_DWORDS: u64 = 0x0000_1260;
const NAVI21_SDMA0_BASE0_DWORDS: u64 = 0x0000_1260;
const NAVI21_GRBM_STATUS: u32 = 0x0da4;
const NAVI21_GRBM_CHIP_REVISION: u32 = 0x0dc1;
const NAVI21_SDMA0_STATUS: u32 = 0x0025;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BaseSource { None=0, AmdGeneratedOffsetHeader=1, IpDiscoveryRecord=2 }

#[derive(Clone, Copy, Debug)]
pub struct DiscoveryIpRecord {
    pub ip: RegisterIp,
    pub instance: u8,
    pub major: u8,
    pub minor: u8,
    pub revision: u8,
    pub base0_dwords: u64,
    pub trusted_checksum: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct TrustedIpBaseMap {
    pub source: BaseSource,
    pub gc_base_dwords: u64,
    pub sdma0_base_dwords: u64,
    pub gc_valid: bool,
    pub sdma0_valid: bool,
}
impl TrustedIpBaseMap {
    pub const EMPTY:Self=Self{source:BaseSource::None,gc_base_dwords:0,sdma0_base_dwords:0,gc_valid:false,sdma0_valid:false};
    pub const NAVI21:Self=Self{source:BaseSource::AmdGeneratedOffsetHeader,gc_base_dwords:NAVI21_GC_BASE0_DWORDS,sdma0_base_dwords:NAVI21_SDMA0_BASE0_DWORDS,gc_valid:true,sdma0_valid:true};
}

#[derive(Clone, Copy, Debug)]
pub struct C12State {
    pub amd_present: bool,
    pub profile_verified: bool,
    pub exact_domain_live: bool,
    pub base_source: BaseSource,
    pub gc_base_ready: bool,
    pub sdma_base_ready: bool,
    pub register_mmio_bar_ready: bool,
    pub live_read_gate_ready: bool,
    pub live_reads_performed: u8,
    pub grbm_status_valid: bool,
    pub chip_revision_valid: bool,
    pub sdma_status_valid: bool,
    pub grbm_status: u32,
    pub chip_revision: u32,
    pub sdma_status: u32,
    pub write_path_fenced: bool,
    pub firmware_upload_enabled: bool,
    pub command_submit_enabled: bool,
    pub bus_master_enabled: bool,
    pub fallback_armed: bool,
    pub device_id: u16,
    pub revision: u8,
}
impl C12State { pub const EMPTY:Self=Self{
    amd_present:false,profile_verified:false,exact_domain_live:false,base_source:BaseSource::None,
    gc_base_ready:false,sdma_base_ready:false,register_mmio_bar_ready:false,live_read_gate_ready:false,
    live_reads_performed:0,grbm_status_valid:false,chip_revision_valid:false,sdma_status_valid:false,
    grbm_status:0,chip_revision:0,sdma_status:0,write_path_fenced:true,firmware_upload_enabled:false,
    command_submit_enabled:false,bus_master_enabled:false,fallback_armed:true,device_id:0,revision:0,
}; }
static STATE:SpinLock<C12State>=SpinLock::new(C12State::EMPTY);

fn map_from_discovery_records(records:&[DiscoveryIpRecord])->TrustedIpBaseMap{
    let mut map=TrustedIpBaseMap{source:BaseSource::IpDiscoveryRecord,..TrustedIpBaseMap::EMPTY};
    for r in records.iter().take(32) {
        if !r.trusted_checksum || r.instance!=0 || r.base0_dwords==0 { continue; }
        match r.ip {
            RegisterIp::Gc if !map.gc_valid => { map.gc_base_dwords=r.base0_dwords; map.gc_valid=true; }
            RegisterIp::Sdma if !map.sdma0_valid => { map.sdma0_base_dwords=r.base0_dwords; map.sdma0_valid=true; }
            _ => {}
        }
    }
    if !(map.gc_valid || map.sdma0_valid) { map.source=BaseSource::None; }
    map
}

fn map_for_profile(profile:native_gpu_c9::ProfileId)->TrustedIpBaseMap{
    match profile {
        native_gpu_c9::ProfileId::Navi21Rx6800_6900 => TrustedIpBaseMap::NAVI21,
        // Navi 48 deliberately requires trusted IP-discovery input; C12 does
        // not invent a static base map for GFX12 hardware.
        native_gpu_c9::ProfileId::Navi48Rx9070 => TrustedIpBaseMap::EMPTY,
        _ => TrustedIpBaseMap::EMPTY,
    }
}

fn resolve_dword(map:TrustedIpBaseMap,ip:RegisterIp,reg:u32)->Option<u64>{
    let base=match ip { RegisterIp::Gc if map.gc_valid=>map.gc_base_dwords, RegisterIp::Sdma if map.sdma0_valid=>map.sdma0_base_dwords, _=>return None };
    base.checked_add(reg as u64)
}

fn register_mmio_bar()->Option<u64>{
    let b=native_gpu_binding::state();
    pci::memory_bar_base(
        crate::pci::PciFunction{bus:b.selected_bus,device:b.selected_device,function:b.selected_function,
            vendor_id:0,device_id:0,class_code:0,subclass:0,programming_interface:0,revision:0,header_type:0},
        RADEON_C12_MMIO_BAR_INDEX)
}

unsafe fn read_ro_u32_page(allocator:&mut FrameAllocator<'_>,kernel_cr3:u64,bar_phys:u64,byte_offset:u64)->Result<u32,&'static str>{
    if byte_offset > 0x00ff_ffff { return Err("K14.C12 register offset outside bounded direct-MMIO window"); }
    let page_offset=byte_offset & !(RADEON_C12_PAGE_BYTES-1);
    let in_page=byte_offset & (RADEON_C12_PAGE_BYTES-1);
    if in_page+4>RADEON_C12_PAGE_BYTES || in_page&3!=0 { return Err("K14.C12 unaligned/cross-page register read"); }
    let phys=bar_phys.checked_add(page_offset).ok_or("K14.C12 MMIO physical overflow")?;
    let virt=paging::map_kernel_mmio_readonly(allocator,kernel_cr3,phys,RADEON_C12_PAGE_BYTES)?;
    Ok(unsafe{core::ptr::read_volatile((virt+in_page) as *const u32)})
}

fn self_test()->Result<(),&'static str>{
    if K14C12_ABI_VERSION!=1 || RADEON_C12_MMIO_BAR_INDEX!=5 || RADEON_C12_PAGE_BYTES!=4096
        || RADEON_C12_MAX_LIVE_READS<3 || RADEON_C12_MMIO_WRITES_ALLOWED || RADEON_C12_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C12_COMMAND_SUBMIT_ALLOWED || RADEON_C12_BUS_MASTER_ALLOWED { return Err("K14.C12 fail-closed constants invalid"); }
    let map=TrustedIpBaseMap::NAVI21;
    if resolve_dword(map,RegisterIp::Gc,NAVI21_GRBM_STATUS)!=Some(0x2004)
        || resolve_dword(map,RegisterIp::Sdma,NAVI21_SDMA0_STATUS)!=Some(0x1285) { return Err("K14.C12 Navi21 base resolver failed"); }
    let records=[
        DiscoveryIpRecord{ip:RegisterIp::Gc,instance:0,major:12,minor:0,revision:1,base0_dwords:0x4000,trusted_checksum:true},
        DiscoveryIpRecord{ip:RegisterIp::Sdma,instance:0,major:7,minor:0,revision:1,base0_dwords:0x5000,trusted_checksum:true},
    ];
    let parsed=map_from_discovery_records(&records);
    if parsed.source!=BaseSource::IpDiscoveryRecord || !parsed.gc_valid || !parsed.sdma0_valid || parsed.gc_base_dwords!=0x4000 || parsed.sdma0_base_dwords!=0x5000 {
        return Err("K14.C12 trusted discovery-record selector failed");
    }
    Ok(())
}

pub fn initialize(allocator:&mut FrameAllocator<'_>,kernel_cr3:u64)->Result<C12State,&'static str>{
    self_test()?;
    let c6=native_gpu_c6::state();
    let c9=native_gpu_c9::state();
    let c11=native_gpu_c11::state();
    let mut s=C12State{amd_present:c9.amd_present,profile_verified:c9.profile_verified,exact_domain_live:c6.persistent_domain_live,
        device_id:c9.device_id,revision:c9.revision,..C12State::EMPTY};

    let map=map_for_profile(c9.profile);
    s.base_source=map.source; s.gc_base_ready=map.gc_valid; s.sdma_base_ready=map.sdma0_valid;

    serial::println(format_args!("[C12BS] trusted Radeon IP-base source: navi21=AMD_sienna_cichlid_offset_header navi48=IP_discovery_required discovery_checksummed=true guessed_bases=false"));
    serial::println(format_args!("[C12RM] Radeon register aperture: modern_rmmio_bar=5 page_bytes={} supervisor_only=true read_only=true NX=true uncached=true map_per_register_page=true",RADEON_C12_PAGE_BYTES));
    serial::println(format_args!("[C12LR] live-read sequence: exact_domain -> verified_profile -> trusted_ip_base -> BAR5 -> resolve_dword -> page_map_ro -> bounded_u32_read; writes=false upload=false submit=false bus_master=false"));

    if !c9.amd_present {
        serial::println(format_args!("[C12HW] physical Radeon status reads: present=false qemu_deferred=true base_source=none bar5=false reads=0 writes=false fallback=true"));
    } else if !c9.profile_verified || !c9.pci_identity_consistent || !c11.register_definitions_reviewed || !c6.persistent_domain_live {
        serial::println(format_args!("[C12HW] physical Radeon status reads: present=true devid={:#06x} profile_verified={} identity={} reviewed_defs={} domain_live={} reads=0 reason=prerequisite_not_live fallback=true",
            c9.device_id,c9.profile_verified,c9.pci_identity_consistent,c11.register_definitions_reviewed,c6.persistent_domain_live));
    } else if !(map.gc_valid && map.sdma0_valid) {
        serial::println(format_args!("[C12HW] physical Radeon status reads: present=true devid={:#06x} profile={:?} trusted_bases=false reads=0 reason=ip_discovery_required fallback=true",c9.device_id,c9.profile));
    } else if let Some(bar)=register_mmio_bar() {
        s.register_mmio_bar_ready=true;
        let command=pci::read_u16(native_gpu_binding::state().selected_bus,native_gpu_binding::state().selected_device,native_gpu_binding::state().selected_function,0x04);
        if command&(1<<2)!=0 { return Err("K14.C12 Radeon bus mastering unexpectedly enabled before read proof"); }
        let grbm=resolve_dword(map,RegisterIp::Gc,NAVI21_GRBM_STATUS).ok_or("K14.C12 GRBM base unresolved")?.checked_mul(4).ok_or("K14.C12 GRBM offset overflow")?;
        let chip=resolve_dword(map,RegisterIp::Gc,NAVI21_GRBM_CHIP_REVISION).ok_or("K14.C12 chip-revision base unresolved")?.checked_mul(4).ok_or("K14.C12 chip offset overflow")?;
        let sdma=resolve_dword(map,RegisterIp::Sdma,NAVI21_SDMA0_STATUS).ok_or("K14.C12 SDMA base unresolved")?.checked_mul(4).ok_or("K14.C12 SDMA offset overflow")?;
        s.live_read_gate_ready=true;
        s.grbm_status=unsafe{read_ro_u32_page(allocator,kernel_cr3,bar,grbm)?}; s.grbm_status_valid=true; s.live_reads_performed+=1;
        s.chip_revision=unsafe{read_ro_u32_page(allocator,kernel_cr3,bar,chip)?}; s.chip_revision_valid=true; s.live_reads_performed+=1;
        s.sdma_status=unsafe{read_ro_u32_page(allocator,kernel_cr3,bar,sdma)?}; s.sdma_status_valid=true; s.live_reads_performed+=1;
        serial::println(format_args!("[C12R0] Radeon live read: GRBM_STATUS={:#010x} byte_offset={:#x}",s.grbm_status,grbm));
        serial::println(format_args!("[C12R1] Radeon live read: GRBM_CHIP_REVISION={:#010x} byte_offset={:#x}",s.chip_revision,chip));
        serial::println(format_args!("[C12R2] Radeon live read: SDMA0_STATUS_REG={:#010x} byte_offset={:#x}",s.sdma_status,sdma));
        serial::println(format_args!("[C12HW] physical Radeon status reads: present=true devid={:#06x} profile={:?} base_source={:?} bar5=true reads={} writes=false bus_master=false fallback=true",c9.device_id,c9.profile,s.base_source,s.live_reads_performed));
    } else {
        serial::println(format_args!("[C12HW] physical Radeon status reads: present=true devid={:#06x} trusted_bases=true bar5=false reads=0 reason=register_mmio_bar_missing fallback=true",c9.device_id));
    }

    if s.live_reads_performed>0 && (!s.exact_domain_live || !s.profile_verified || !s.gc_base_ready || !s.sdma_base_ready || !s.register_mmio_bar_ready || !s.live_read_gate_ready) {
        return Err("K14.C12 performed live read without all safety gates");
    }
    if s.live_reads_performed>RADEON_C12_MAX_LIVE_READS { return Err("K14.C12 exceeded bounded live-read count"); }
    if !s.write_path_fenced || s.firmware_upload_enabled || s.command_submit_enabled || s.bus_master_enabled {
        return Err("K14.C12 destructive Radeon capability promoted early");
    }
    serial::println(format_args!("[C12RD] K14.C12 trusted IP-base/live-read path ready: amd_present={} profile_verified={} domain_live={} base_source={:?} gc_base={} sdma_base={} bar5={} live_gate={} reads={} grbm={} chiprev={} sdma={} writes=false upload=false submit=false bus_master=false fallback=true",
        s.amd_present,s.profile_verified,s.exact_domain_live,s.base_source,s.gc_base_ready,s.sdma_base_ready,s.register_mmio_bar_ready,s.live_read_gate_ready,s.live_reads_performed,
        s.grbm_status_valid,s.chip_revision_valid,s.sdma_status_valid));
    *STATE.lock()=s; Ok(s)
}

pub fn state()->C12State{*STATE.lock()}
pub fn packed_status()->u64{
    let s=state(); let mut v=(u64::from(s.device_id)<<32)|(u64::from(s.revision)<<24)|(u64::from(s.live_reads_performed)<<16)|(u64::from(s.base_source as u8)<<14);
    for (bit,on) in [s.amd_present,s.profile_verified,s.exact_domain_live,s.gc_base_ready,s.sdma_base_ready,s.register_mmio_bar_ready,s.live_read_gate_ready,
        s.grbm_status_valid,s.chip_revision_valid,s.sdma_status_valid,s.write_path_fenced,s.firmware_upload_enabled,s.command_submit_enabled,s.bus_master_enabled,s.fallback_armed]
        .into_iter().enumerate(){if on{v|=1u64<<bit;}} v
}
