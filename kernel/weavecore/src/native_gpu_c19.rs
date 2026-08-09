//! K14.C19 bounded physical AMD IP-discovery snapshot acquisition.
//!
//! C19 opens the first physical discovery-snapshot read path, but only through
//! CPU read-only mappings.  It imports AMDGPU's source-backed discovery-TMR
//! location contract (DRIVER_SCRATCH_0/1/2 with the VRAM-tail fallback), reads
//! the current BAR0 visible-VRAM aperture size from the PCIe Resizable BAR
//! extended capability, and only maps the TMR when the complete snapshot fits
//! inside that already-visible aperture.  No MM_INDEX/MM_DATA fallback is used
//! because that path requires MMIO writes.  Bus mastering stays OFF.
//!
//! A physical snapshot is accepted only after C18 verifies both the AMD binary
//! checksum and the IP_DISCOVERY table checksum.  C19 does not yet resolve GC
//! or SDMA bases from the verified snapshot; that is the next narrow stage.

use crate::{
    acpi,
    memory::FrameAllocator,
    native_gpu_c6,
    native_gpu_c9,
    native_gpu_c18,
    native_gpu_binding,
    paging,
    pci::{self, PciFunction},
    pci_address::PciAddress,
    pci_ecam::EcamCatalog,
    serial,
    sync::SpinLock,
};

pub const K14C19_ABI_VERSION: u32 = 1;
pub const RADEON_C19_MMIO_BAR_INDEX: u8 = 5;
pub const RADEON_C19_VRAM_BAR_INDEX: u8 = 0;
pub const RADEON_C19_MAX_SNAPSHOT_BYTES: usize = 64 * 1024;
pub const RADEON_C19_MIN_SNAPSHOT_BYTES: usize = 64;
pub const RADEON_C19_MAX_EXT_CAPS: u8 = 64;
pub const PCI_EXT_CAP_ID_REBAR: u16 = 0x15;
pub const PCI_EXT_CAP_START: u16 = 0x100;
pub const PCI_REBAR_CTRL: u16 = 8;
pub const PCI_REBAR_CTRL_BAR_IDX_MASK: u32 = 0x0000_0007;
pub const PCI_REBAR_CTRL_NBAR_MASK: u32 = 0x0000_00e0;
pub const PCI_REBAR_CTRL_NBAR_SHIFT: u32 = 5;
pub const PCI_REBAR_CTRL_BAR_SIZE_MASK: u32 = 0x0000_1f00;
pub const PCI_REBAR_CTRL_BAR_SIZE_SHIFT: u32 = 8;

// C19 promotes only read-only CPU access.  The generic AMD MM-index fallback
// writes address selector registers and therefore remains prohibited here.
pub const RADEON_C19_DIRECT_VRAM_APERTURE_READ_ALLOWED: bool = true;
pub const RADEON_C19_MM_INDEX_FALLBACK_ALLOWED: bool = false;
pub const RADEON_C19_MMIO_WRITES_ALLOWED: bool = false;
pub const RADEON_C19_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C19_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C19_BUS_MASTER_ALLOWED: bool = false;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TmrLocationSource { None=0, DriverScratch=1, VramTailFallback=2 }

#[derive(Clone, Copy, Debug)]
pub struct C19State {
    pub amd_present: bool,
    pub navi48: bool,
    pub c18_ready: bool,
    pub exact_domain_live: bool,
    pub profile_verified: bool,
    pub memory_decode_on: bool,
    pub bus_master_before_off: bool,
    pub bus_master_after_off: bool,
    pub bar5_ready: bool,
    pub scratch_reads_performed: u8,
    pub tmr_location_source: TmrLocationSource,
    pub tmr_offset: u64,
    pub tmr_size: u32,
    pub ecam_ready: bool,
    pub rebar_found: bool,
    pub bar0_ready: bool,
    pub bar0_aperture_bytes: u64,
    pub aperture_covers_tmr: bool,
    pub live_snapshot_acquired: bool,
    pub live_binary_checksum_verified: bool,
    pub live_ip_checksum_verified: bool,
    pub live_snapshot_verified: bool,
    pub snapshot_bytes: u32,
    pub snapshot_fingerprint: u64,
    pub exact_gc_base_resolved: bool,
    pub mm_index_fallback_used: bool,
    pub mmio_write_enabled: bool,
    pub firmware_upload_enabled: bool,
    pub command_submit_enabled: bool,
    pub radeon_bus_master_enabled: bool,
    pub fallback_armed: bool,
    pub device_id: u16,
    pub revision: u8,
}
impl C19State {
    pub const EMPTY:Self=Self{
        amd_present:false,navi48:false,c18_ready:false,exact_domain_live:false,profile_verified:false,
        memory_decode_on:false,bus_master_before_off:false,bus_master_after_off:false,bar5_ready:false,
        scratch_reads_performed:0,tmr_location_source:TmrLocationSource::None,tmr_offset:0,tmr_size:0,
        ecam_ready:false,rebar_found:false,bar0_ready:false,bar0_aperture_bytes:0,aperture_covers_tmr:false,
        live_snapshot_acquired:false,live_binary_checksum_verified:false,live_ip_checksum_verified:false,
        live_snapshot_verified:false,snapshot_bytes:0,snapshot_fingerprint:0,exact_gc_base_resolved:false,
        mm_index_fallback_used:false,mmio_write_enabled:false,firmware_upload_enabled:false,
        command_submit_enabled:false,radeon_bus_master_enabled:false,fallback_armed:true,device_id:0,revision:0,
    };
}
static STATE:SpinLock<C19State>=SpinLock::new(C19State::EMPTY);
static SNAPSHOT:SpinLock<[u8; RADEON_C19_MAX_SNAPSHOT_BYTES]>=SpinLock::new([0; RADEON_C19_MAX_SNAPSHOT_BYTES]);

fn selected_function()->Option<PciFunction>{
    let b=native_gpu_binding::state(); let mut found=None;
    pci::enumerate(|f|{if f.bus==b.selected_bus&&f.device==b.selected_device&&f.function==b.selected_function{found=Some(f);}}); found
}

fn le_u32_from(mut read:impl FnMut(u16)->Option<u32>, offset:u16)->Option<u32>{read(offset)}

/// Parse only the read-only information C19 needs from the PCIe Resizable BAR
/// extended capability.  Encoded size 0 is 1 MiB, matching PCIe/Linux rebar.c.
fn bar0_rebar_size(mut read:impl FnMut(u16)->Option<u32>)->Option<u64>{
    let mut pos=PCI_EXT_CAP_START;
    for _ in 0..RADEON_C19_MAX_EXT_CAPS {
        let header=le_u32_from(&mut read,pos)?;
        if header==0||header==u32::MAX{return None;}
        let id=(header&0xffff)as u16;
        if id==PCI_EXT_CAP_ID_REBAR{
            let first_ctrl=le_u32_from(&mut read,pos.checked_add(PCI_REBAR_CTRL)?)?;
            let nbars=((first_ctrl&PCI_REBAR_CTRL_NBAR_MASK)>>PCI_REBAR_CTRL_NBAR_SHIFT)as u16;
            if nbars==0||nbars>6{return None;}
            for i in 0..nbars{
                let ctrl_off=pos.checked_add(PCI_REBAR_CTRL)?.checked_add(i.checked_mul(8)?)?;
                if ctrl_off>4092{return None;}
                let ctrl=le_u32_from(&mut read,ctrl_off)?;
                if ctrl&PCI_REBAR_CTRL_BAR_IDX_MASK==u32::from(RADEON_C19_VRAM_BAR_INDEX){
                    let enc=(ctrl&PCI_REBAR_CTRL_BAR_SIZE_MASK)>>PCI_REBAR_CTRL_BAR_SIZE_SHIFT;
                    let shift=20u32.checked_add(enc)?;
                    if shift>=64{return None;}
                    return Some(1u64<<shift);
                }
            }
            return None;
        }
        let next=((header>>20)&0x0ffc)as u16;
        if next==0{return None;}
        if next<PCI_EXT_CAP_START||next&3!=0||next==pos{return None;}
        pos=next;
    }
    None
}

fn resolve_tmr(s0:u32,s1:u32,s2:u32,vram_mib:u32)->Option<(TmrLocationSource,u64,u32)>{
    if s2!=0&&s2!=u32::MAX{
        let size=s2;
        if (size as usize)<RADEON_C19_MIN_SNAPSHOT_BYTES||(size as usize)>RADEON_C19_MAX_SNAPSHOT_BYTES{return None;}
        let offset=(u64::from(s1)<<32)|u64::from(s0);
        if offset==0{return None;}
        return Some((TmrLocationSource::DriverScratch,offset,size));
    }
    if vram_mib==0||vram_mib==u32::MAX{return None;}
    let bytes=u64::from(vram_mib).checked_shl(20)?;
    let offset=bytes.checked_sub(native_gpu_c18::AMD_DISCOVERY_TMR_OFFSET)?;
    Some((TmrLocationSource::VramTailFallback,offset,native_gpu_c18::AMD_DISCOVERY_TMR_SIZE))
}

unsafe fn read_bar5_u32(allocator:&mut FrameAllocator<'_>,cr3:u64,bar5:u64,dword:u32)->Result<u32,&'static str>{
    let byte=u64::from(dword).checked_mul(4).ok_or("K14.C19 register offset overflow")?;
    if byte>0x00ff_ffff{return Err("K14.C19 register outside bounded direct-MMIO window");}
    let page=byte&!4095; let in_page=byte&4095;
    if in_page+4>4096||in_page&3!=0{return Err("K14.C19 invalid register alignment");}
    let phys=bar5.checked_add(page).ok_or("K14.C19 BAR5 physical overflow")?;
    let virt=paging::map_kernel_mmio_readonly(allocator,cr3,phys,4096)?;
    Ok(unsafe{core::ptr::read_volatile((virt+in_page)as *const u32)})
}

fn ecam_config_mapping(allocator:&mut FrameAllocator<'_>,cr3:u64,rsdp:u64,identity_limit:u64,function:PciFunction)->Result<u64,&'static str>{
    let acpi=acpi::AcpiCatalog::build(rsdp,identity_limit)?;
    let ecam=EcamCatalog::from_acpi(&acpi)?;
    let addr=PciAddress::new(0,function.bus,function.device,function.function)?;
    let w=ecam.window(addr).ok_or("K14.C19 no ECAM window for selected Radeon")?;
    let phys=w.base
        .checked_add((u64::from(function.bus-w.start_bus))<<20).ok_or("K14.C19 ECAM bus overflow")?
        .checked_add(u64::from(function.device)<<15).ok_or("K14.C19 ECAM device overflow")?
        .checked_add(u64::from(function.function)<<12).ok_or("K14.C19 ECAM function overflow")?;
    paging::map_kernel_mmio_readonly(allocator,cr3,phys,4096)
}

fn rebar_size_from_ecam(config_virt:u64)->Option<u64>{
    bar0_rebar_size(|off|{
        if off>4092||off&3!=0{return None;}
        Some(unsafe{core::ptr::read_volatile((config_virt+u64::from(off))as *const u32)})
    })
}

unsafe fn copy_vram_snapshot(allocator:&mut FrameAllocator<'_>,cr3:u64,bar0:u64,offset:u64,size:usize)->Result<native_gpu_c18::SnapshotVerification,&'static str>{
    if size<RADEON_C19_MIN_SNAPSHOT_BYTES { return Err("K14.C19 snapshot below minimum size"); }
    let end=offset.checked_add(size as u64).ok_or("K14.C19 snapshot offset overflow")?;
    let phys=bar0.checked_add(offset).ok_or("K14.C19 BAR0 snapshot physical overflow")?;
    let _=end;
    let virt=paging::map_kernel_mmio_readonly(allocator,cr3,phys,size as u64)?;
    let mut snap=SNAPSHOT.lock();
    for i in 0..size{snap[i]=unsafe{core::ptr::read_volatile((virt+i as u64)as *const u8)};}
    native_gpu_c18::verify_snapshot_checksums(&snap[..size])
}

fn self_test()->Result<(),&'static str>{
    if K14C19_ABI_VERSION!=1||RADEON_C19_MMIO_BAR_INDEX!=5||RADEON_C19_VRAM_BAR_INDEX!=0
        ||RADEON_C19_MAX_SNAPSHOT_BYTES<native_gpu_c18::AMD_DISCOVERY_TMR_SIZE as usize
        ||!RADEON_C19_DIRECT_VRAM_APERTURE_READ_ALLOWED||RADEON_C19_MM_INDEX_FALLBACK_ALLOWED
        ||RADEON_C19_MMIO_WRITES_ALLOWED||RADEON_C19_FIRMWARE_UPLOAD_ALLOWED
        ||RADEON_C19_COMMAND_SUBMIT_ALLOWED||RADEON_C19_BUS_MASTER_ALLOWED{return Err("K14.C19 fail-closed constants invalid");}
    let mut cfg=[0u8;4096];
    // One ReBAR capability at 0x100, one BAR entry, BAR0, encoded size 14 => 16 GiB.
    cfg[0x100..0x104].copy_from_slice(&(u32::from(PCI_EXT_CAP_ID_REBAR)|1u32<<16).to_le_bytes());
    let ctrl=(1u32<<PCI_REBAR_CTRL_NBAR_SHIFT)|(14u32<<PCI_REBAR_CTRL_BAR_SIZE_SHIFT);
    cfg[0x108..0x10c].copy_from_slice(&ctrl.to_le_bytes());
    let got=bar0_rebar_size(|off|{let o=off as usize;if o+4>cfg.len(){None}else{Some(u32::from_le_bytes([cfg[o],cfg[o+1],cfg[o+2],cfg[o+3]]))}});
    if got!=Some(16u64<<30){return Err("K14.C19 ReBAR parser self-test failed");}
    let t=resolve_tmr(0x1234_0000,0,10*1024,16*1024).ok_or("K14.C19 scratch TMR self-test failed")?;
    if t.0!=TmrLocationSource::DriverScratch||t.1!=0x1234_0000||t.2!=10*1024{return Err("K14.C19 scratch TMR self-test mismatch");}
    let f=resolve_tmr(0,0,0,16*1024).ok_or("K14.C19 tail TMR self-test failed")?;
    if f.0!=TmrLocationSource::VramTailFallback||f.1!=((16u64<<30)-native_gpu_c18::AMD_DISCOVERY_TMR_OFFSET){return Err("K14.C19 tail TMR self-test mismatch");}
    Ok(())
}

pub fn initialize(allocator:&mut FrameAllocator<'_>,cr3:u64,rsdp:u64,identity_limit:u64)->Result<C19State,&'static str>{
    self_test()?;
    let c6=native_gpu_c6::state(); let c9=native_gpu_c9::state(); let c18=native_gpu_c18::state();
    let mut s=C19State{
        amd_present:c9.amd_present,navi48:c9.profile==native_gpu_c9::ProfileId::Navi48Rx9070,
        c18_ready:c18.checksum_engine_ready&&c18.tmr_contract_imported&&c18.synthetic_checksum_selftest_passed,
        exact_domain_live:c6.persistent_domain_live,profile_verified:c9.profile_verified,
        device_id:c9.device_id,revision:c9.revision,..C19State::EMPTY
    };
    serial::println(format_args!("[C19PG] physical discovery-read policy: source=AMD_TMR require=C18_verified_engine+exact_domain+profile+BAR5_scratch+ECAM_ReBAR+BAR0_full_coverage+bus_master_off direct_aperture_read=true MM_INDEX_fallback=false MMIO_write=false firmware=false submit=false bus_master_enable=false"));
    serial::println(format_args!("[C19AP] VRAM aperture contract: BAR0=current_Resizable_BAR_size read_only=true snapshot_max={} checksum_required=binary+IP stale_or_partial_snapshot=rejected",RADEON_C19_MAX_SNAPSHOT_BYTES));

    if !s.amd_present{
        serial::println(format_args!("[C19HW] physical AMD discovery snapshot: present=false qemu_deferred=true scratch_reads=0 rebar=false aperture=false acquired=false verified=false fallback=true"));
    } else {
        let f=selected_function().ok_or("K14.C19 selected Radeon disappeared")?;
        let cmd=pci::read_u16(f.bus,f.device,f.function,0x04);
        s.memory_decode_on=cmd&(1<<1)!=0; s.bus_master_before_off=cmd&(1<<2)==0;
        if !s.bus_master_before_off{return Err("K14.C19 Radeon bus mastering unexpectedly enabled");}
        if !(s.c18_ready&&s.exact_domain_live&&s.profile_verified&&s.memory_decode_on){
            serial::println(format_args!("[C19HW] physical AMD discovery snapshot: present=true devid={:#06x} prerequisites=false domain={} profile={} memory_decode={} acquired=false reason=prerequisite_not_live fallback=true",s.device_id,s.exact_domain_live,s.profile_verified,s.memory_decode_on));
        } else if let Some(bar5)=pci::memory_bar_base(f,RADEON_C19_MMIO_BAR_INDEX){
            s.bar5_ready=true;
            let sr0=unsafe{read_bar5_u32(allocator,cr3,bar5,native_gpu_c18::AMD_DISCOVERY_MM_DRIVER_SCRATCH_0)?};
            let sr1=unsafe{read_bar5_u32(allocator,cr3,bar5,native_gpu_c18::AMD_DISCOVERY_MM_DRIVER_SCRATCH_1)?};
            let sr2=unsafe{read_bar5_u32(allocator,cr3,bar5,native_gpu_c18::AMD_DISCOVERY_MM_DRIVER_SCRATCH_2)?};
            let vram=unsafe{read_bar5_u32(allocator,cr3,bar5,native_gpu_c18::AMD_DISCOVERY_MM_RCC_CONFIG_MEMSIZE)?};
            s.scratch_reads_performed=4;
            if let Some((source,off,size))=resolve_tmr(sr0,sr1,sr2,vram){
                s.tmr_location_source=source;s.tmr_offset=off;s.tmr_size=size;
                match ecam_config_mapping(allocator,cr3,rsdp,identity_limit,f){
                    Ok(config)=>{
                        s.ecam_ready=true;
                        if let Some(aperture)=rebar_size_from_ecam(config){
                            s.rebar_found=true;s.bar0_aperture_bytes=aperture;
                            if let Some(bar0)=pci::memory_bar_base(f,RADEON_C19_VRAM_BAR_INDEX){
                                s.bar0_ready=true;
                                let tmr_end=off.checked_add(u64::from(size)).ok_or("K14.C19 TMR range overflow")?;
                                s.aperture_covers_tmr=tmr_end<=aperture;
                                if s.aperture_covers_tmr{
                                    let proof=unsafe{copy_vram_snapshot(allocator,cr3,bar0,off,size as usize)?};
                                    s.live_snapshot_acquired=true;s.live_binary_checksum_verified=true;s.live_ip_checksum_verified=true;
                                    s.live_snapshot_verified=proof.valid;s.snapshot_bytes=u32::from(proof.binary_size);s.snapshot_fingerprint=proof.fingerprint;
                                    serial::println(format_args!("[C19HW] physical AMD discovery snapshot: present=true devid={:#06x} source={:?} tmr_offset={:#x} tmr_size={} rebar=true aperture={} acquired=true binary_ck=true ip_ck=true verified={} fingerprint={:#018x} fallback=true",s.device_id,s.tmr_location_source,s.tmr_offset,s.tmr_size,s.bar0_aperture_bytes,s.live_snapshot_verified,s.snapshot_fingerprint));
                                }else{serial::println(format_args!("[C19HW] physical AMD discovery snapshot: present=true devid={:#06x} source={:?} tmr_offset={:#x} tmr_size={} aperture={} acquired=false reason=TMR_outside_visible_BAR0 fallback=true",s.device_id,s.tmr_location_source,s.tmr_offset,s.tmr_size,s.bar0_aperture_bytes));}
                            }else{serial::println(format_args!("[C19HW] physical AMD discovery snapshot: present=true devid={:#06x} acquired=false reason=BAR0_missing fallback=true",s.device_id));}
                        }else{serial::println(format_args!("[C19HW] physical AMD discovery snapshot: present=true devid={:#06x} ECAM=true acquired=false reason=BAR0_ReBAR_size_unavailable fallback=true",s.device_id));}
                    }
                    Err(_)=>serial::println(format_args!("[C19HW] physical AMD discovery snapshot: present=true devid={:#06x} acquired=false reason=ECAM_MCFG_unavailable fallback=true",s.device_id)),
                }
            }else{serial::println(format_args!("[C19HW] physical AMD discovery snapshot: present=true devid={:#06x} scratch_reads={} acquired=false reason=TMR_location_unresolved fallback=true",s.device_id,s.scratch_reads_performed));}
        }else{serial::println(format_args!("[C19HW] physical AMD discovery snapshot: present=true devid={:#06x} acquired=false reason=BAR5_missing fallback=true",s.device_id));}
        let cmd2=pci::read_u16(f.bus,f.device,f.function,0x04);s.bus_master_after_off=cmd2&(1<<2)==0;
        if !s.bus_master_after_off{return Err("K14.C19 bus mastering changed during read-only acquisition");}
    }
    if s.live_snapshot_verified&&!(s.live_snapshot_acquired&&s.live_binary_checksum_verified&&s.live_ip_checksum_verified&&s.aperture_covers_tmr){return Err("K14.C19 snapshot verified without complete acquisition gates");}
    if s.live_snapshot_acquired&&s.snapshot_fingerprint==0{return Err("K14.C19 acquired snapshot without fingerprint");}
    if s.mm_index_fallback_used||s.mmio_write_enabled||s.firmware_upload_enabled||s.command_submit_enabled||s.radeon_bus_master_enabled{return Err("K14.C19 destructive capability promoted early");}
    serial::println(format_args!("[C19RD] K14.C19 physical snapshot gate ready: amd_present={} navi48={} C18_ready={} domain={} profile={} memdecode={} bar5={} scratch_reads={} source={:?} ECAM={} rebar={} bar0={} aperture={} covers={} acquired={} binary_ck={} ip_ck={} verified={} bytes={} fingerprint={:#018x} MM_INDEX=false writes=false upload=false submit=false bus_master=false fallback=true",s.amd_present,s.navi48,s.c18_ready,s.exact_domain_live,s.profile_verified,s.memory_decode_on,s.bar5_ready,s.scratch_reads_performed,s.tmr_location_source,s.ecam_ready,s.rebar_found,s.bar0_ready,s.bar0_aperture_bytes,s.aperture_covers_tmr,s.live_snapshot_acquired,s.live_binary_checksum_verified,s.live_ip_checksum_verified,s.live_snapshot_verified,s.snapshot_bytes,s.snapshot_fingerprint));
    *STATE.lock()=s;Ok(s)
}

pub fn state()->C19State{*STATE.lock()}

pub fn with_verified_snapshot<R>(f:impl FnOnce(&[u8])->R)->Option<R>{
    let s=state();if !s.live_snapshot_verified||s.snapshot_bytes==0{return None;}
    let snap=SNAPSHOT.lock();Some(f(&snap[..s.snapshot_bytes as usize]))
}

pub fn packed_status()->u64{
    let s=state();let mut v=(u64::from(s.device_id)<<32)|(u64::from(s.revision)<<24)|(u64::from(s.scratch_reads_performed)<<16);
    for(bit,on)in[s.amd_present,s.navi48,s.c18_ready,s.exact_domain_live,s.profile_verified,s.memory_decode_on,s.bus_master_before_off,s.bus_master_after_off,s.bar5_ready,s.ecam_ready,s.rebar_found,s.bar0_ready,s.aperture_covers_tmr,s.live_snapshot_acquired,s.live_binary_checksum_verified,s.live_ip_checksum_verified,s.live_snapshot_verified,s.mm_index_fallback_used,s.fallback_armed].into_iter().enumerate(){if on{v|=1u64<<bit;}}v
}
