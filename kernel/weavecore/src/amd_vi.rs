//! AMD-Vi (IOMMU) IVRS discovery, command/event rings and translated-domain backend.
use crate::{acpi::{self,AcpiCatalog},iommu_core::{Access,PageSpan,TranslationBackend},pci_address::RequesterId};
pub const MAX_AMD_IOMMUS:usize=8;pub const AMD_RING_ENTRIES:usize=256;
#[derive(Clone,Copy,Debug)]pub struct AmdIommuUnit{pub segment:u16,pub requester:RequesterId,pub mmio:u64,pub flags:u8,pub enabled:bool}impl AmdIommuUnit{pub const EMPTY:Self=Self{segment:0,requester:RequesterId(0),mmio:0,flags:0,enabled:false};}
#[derive(Clone,Copy)]struct Command{lo:u64,hi:u64}impl Command{const EMPTY:Self=Self{lo:0,hi:0};}
pub struct AmdVi{pub units:[AmdIommuUnit;MAX_AMD_IOMMUS],pub count:usize,commands:[Command;AMD_RING_ENTRIES],head:u16,tail:u16,completion:u64,blocked:[bool;65536]}
impl AmdVi{
 pub const fn empty()->Self{Self{units:[AmdIommuUnit::EMPTY;MAX_AMD_IOMMUS],count:0,commands:[Command::EMPTY;AMD_RING_ENTRIES],head:0,tail:0,completion:0,blocked:[true;65536]}}
 pub fn discover(c:&AcpiCatalog)->Result<Self,&'static str>{let t=c.find(*b"IVRS",0).ok_or("IVRS not found")?;if t.length<48{return Err("IVRS too short")}let mut s=Self::empty();let mut off=48usize;while off+4<=t.length as usize{let ty=acpi::read_table_u8(t,off)?;let len=acpi::read_table_u16(t,off+2)? as usize;if len<4||off+len>t.length as usize{return Err("invalid IVRS block")}if matches!(ty,0x10|0x11|0x40){if len<24{return Err("IVHD block too short")}if s.count>=MAX_AMD_IOMMUS{return Err("too many AMD IOMMUs")}let rid=RequesterId(acpi::read_table_u16(t,off+4)?);let mmio=acpi::read_table_u64(t,off+8)?;if mmio==0||mmio&0x3fff!=0{return Err("invalid AMD-Vi MMIO base")}s.units[s.count]=AmdIommuUnit{segment:0,requester:rid,mmio,flags:acpi::read_table_u8(t,off+1)?,enabled:false};s.count+=1}off+=len}if s.count==0{return Err("IVRS contains no IVHD units")}Ok(s)}
 fn push(&mut self,c:Command)->Result<(),&'static str>{let next=self.tail.wrapping_add(1)%(AMD_RING_ENTRIES as u16);if next==self.head{return Err("AMD-Vi command ring full")}self.commands[self.tail as usize]=c;self.tail=next;Ok(())}
 pub fn complete_commands(&mut self){self.head=self.tail;self.completion=self.completion.wrapping_add(1)}
 pub fn enable_units(&mut self)->Result<(),&'static str>{for u in &mut self.units[..self.count]{if u.mmio==0{return Err("AMD-Vi unit missing MMIO")}u.enabled=true}Ok(())}
}
impl TranslationBackend for AmdVi{
 fn attach(&mut self,r:RequesterId,d:u16)->Result<(),&'static str>{self.push(Command{lo:(r.0 as u64)<<32|d as u64,hi:1})?;self.blocked[r.0 as usize]=false;self.complete_commands();Ok(())}
 fn detach(&mut self,r:RequesterId)->Result<(),&'static str>{self.block_requester(r)}
 fn map_pages(&mut self,d:u16,iova:u64,p:&[PageSpan],a:Access)->Result<(),&'static str>{let pages=p.iter().fold(0u64,|a,x|a.saturating_add(x.pages as u64));if pages==0{return Err("empty AMD-Vi mapping")}self.push(Command{lo:iova|d as u64,hi:pages|((matches!(a,Access::Write|Access::ReadWrite)as u64)<<63)})?;Ok(())}
 fn unmap_pages(&mut self,d:u16,iova:u64,pages:u32)->Result<(),&'static str>{self.push(Command{lo:iova|d as u64,hi:(pages as u64)<<32})}
 fn invalidate_domain(&mut self,d:u16)->Result<(),&'static str>{self.push(Command{lo:d as u64,hi:0x3})?;self.complete_commands();Ok(())}
 fn block_requester(&mut self,r:RequesterId)->Result<(),&'static str>{self.blocked[r.0 as usize]=true;self.push(Command{lo:(r.0 as u64)<<32,hi:0})?;self.complete_commands();Ok(())}
}


// --- K14.C5 hardware page-table image foundation -------------------------
use crate::memory::{FrameAllocator, FRAME_SIZE};
use core::ptr;

pub const AMDVI_DTE_BYTES: usize = 32;
pub const AMDVI_COMMAND_BYTES: usize = 16;
pub const AMDVI_EVENT_BYTES: usize = 16;
pub const AMDVI_DEVICE_TABLE_PAGES: u64 = 512; // 65536 requesters * 32 B = 2 MiB
pub const AMDVI_RING_PAGES: u64 = 1;
pub const AMDVI_PTE_PRESENT: u64 = 1 << 0;
pub const AMDVI_PTE_READ: u64 = 1 << 1;
pub const AMDVI_PTE_WRITE: u64 = 1 << 2;
pub const AMDVI_ADDR_MASK: u64 = 0x000f_ffff_ffff_f000;

#[derive(Clone, Copy, Debug)]
pub struct AmdViDomainImage {
    pub device_table: u64,
    pub page_table_root: u64,
    pub command_buffer: u64,
    pub event_log: u64,
}

impl AmdViDomainImage {
    pub fn allocate(allocator: &mut FrameAllocator<'_>, _requester: RequesterId, _domain: u16) -> Result<Self, &'static str> {
        let device_table = allocator.allocate_contiguous(AMDVI_DEVICE_TABLE_PAGES).ok_or("AMD-Vi device-table allocation failed")?;
        let page_table_root = match allocator.allocate_contiguous(1) {
            Some(p) => p, None => { let _=allocator.deallocate_contiguous(device_table, AMDVI_DEVICE_TABLE_PAGES); return Err("AMD-Vi page-table allocation failed"); }
        };
        let command_buffer = match allocator.allocate_contiguous(AMDVI_RING_PAGES) {
            Some(p) => p, None => { let _=allocator.deallocate_contiguous(page_table_root,1); let _=allocator.deallocate_contiguous(device_table,AMDVI_DEVICE_TABLE_PAGES); return Err("AMD-Vi command-buffer allocation failed"); }
        };
        let event_log = match allocator.allocate_contiguous(AMDVI_RING_PAGES) {
            Some(p) => p, None => { let _=allocator.deallocate_contiguous(command_buffer,AMDVI_RING_PAGES); let _=allocator.deallocate_contiguous(page_table_root,1); let _=allocator.deallocate_contiguous(device_table,AMDVI_DEVICE_TABLE_PAGES); return Err("AMD-Vi event-log allocation failed"); }
        };
        for (base,pages) in [(device_table,AMDVI_DEVICE_TABLE_PAGES),(page_table_root,1),(command_buffer,AMDVI_RING_PAGES),(event_log,AMDVI_RING_PAGES)] { zero_pages(base,pages); }
        Ok(Self{device_table,page_table_root,command_buffer,event_log})
    }

    pub fn install_exact_requester_dte(self, requester: RequesterId, domain: u16) -> Result<(), &'static str> {
        if self.device_table & (FRAME_SIZE-1) != 0 || self.page_table_root & (FRAME_SIZE-1) != 0 { return Err("AMD-Vi C5 table image is not page aligned"); }
        let offset=(requester.0 as u64).checked_mul(AMDVI_DTE_BYTES as u64).ok_or("AMD-Vi DTE offset overflow")?;
        if offset + AMDVI_DTE_BYTES as u64 > AMDVI_DEVICE_TABLE_PAGES*FRAME_SIZE { return Err("AMD-Vi requester exceeds device table"); }
        // C5 records the exact requester's translation root and domain in a
        // canonical software image. Hardware-specific reserved/feature bits
        // remain zero until the physical unit is version-qualified.
        let dte=self.device_table+offset;
        unsafe {
            ptr::write_volatile(dte as *mut u64, (self.page_table_root & AMDVI_ADDR_MASK) | AMDVI_PTE_PRESENT | AMDVI_PTE_READ | AMDVI_PTE_WRITE);
            ptr::write_volatile((dte+8) as *mut u64, (domain as u64) << 16);
            ptr::write_volatile((dte+16) as *mut u64, 0);
            ptr::write_volatile((dte+24) as *mut u64, 0);
        }
        Ok(())
    }
}

fn zero_pages(base:u64,pages:u64){
    let words=(pages*FRAME_SIZE/8) as usize;
    unsafe{for i in 0..words{ptr::write_volatile((base as *mut u64).add(i),0)}}
}

pub fn c5_layout_self_test()->Result<(),&'static str>{
    if AMDVI_DTE_BYTES!=32 || AMDVI_COMMAND_BYTES!=16 || AMDVI_EVENT_BYTES!=16 { return Err("AMD-Vi C5 entry sizes invalid"); }
    if AMDVI_DEVICE_TABLE_PAGES*FRAME_SIZE != 65536u64*AMDVI_DTE_BYTES as u64 { return Err("AMD-Vi C5 device-table span invalid"); }
    if AMDVI_ADDR_MASK & 0xfff != 0 { return Err("AMD-Vi C5 address mask invalid"); }
    Ok(())
}


// --- K14.C6 live AMD-Vi hardware programming boundary ------------------
//
// These offsets follow the AMD-Vi MMIO register layout used by the IVHD
// programming model. C6 deliberately keeps physical activation behind an
// explicit bare-metal qualification gate; QEMU has no AMD-Vi/Radeon pair.
pub const AMDVI_MMIO_BYTES: u64 = 0x4000;
pub const AMDVI_REG_DEVICE_TABLE_BASE: u64 = 0x0000;
pub const AMDVI_REG_COMMAND_BUFFER_BASE: u64 = 0x0008;
pub const AMDVI_REG_EVENT_LOG_BASE: u64 = 0x0010;
pub const AMDVI_REG_CONTROL: u64 = 0x0018;
pub const AMDVI_REG_COMMAND_HEAD: u64 = 0x2000;
pub const AMDVI_REG_COMMAND_TAIL: u64 = 0x2008;
pub const AMDVI_REG_EVENT_HEAD: u64 = 0x2010;
pub const AMDVI_REG_EVENT_TAIL: u64 = 0x2018;
pub const AMDVI_REG_STATUS: u64 = 0x2020;

pub const AMDVI_CONTROL_IOMMU_ENABLE: u64 = 1 << 0;
pub const AMDVI_CONTROL_EVENT_LOG_ENABLE: u64 = 1 << 2;
pub const AMDVI_CONTROL_COMMAND_BUFFER_ENABLE: u64 = 1 << 12;

#[derive(Clone, Copy, Debug)]
pub struct AmdViHardwarePlan {
    pub register_base: u64,
    pub device_table: u64,
    pub command_buffer: u64,
    pub event_log: u64,
    pub requester: RequesterId,
    pub domain: u16,
}

impl AmdViHardwarePlan {
    pub fn validate(self) -> Result<(), &'static str> {
        if self.register_base == 0 || self.register_base & 0x3fff != 0 { return Err("AMD-Vi C6 MMIO base invalid"); }
        for p in [self.device_table,self.command_buffer,self.event_log] {
            if p == 0 || p & (FRAME_SIZE-1) != 0 { return Err("AMD-Vi C6 DMA structure alignment invalid"); }
        }
        if self.domain == 0 { return Err("AMD-Vi C6 domain zero is reserved"); }
        Ok(())
    }
}

/// C6 register programming primitive.  The caller must map `register_base`
/// through the kernel MMIO window and must only invoke this after ForgeBus
/// ownership plus exact-requester DTE construction have been proven.
///
/// This routine initializes ring pointers while translation remains disabled,
/// publishes the device/command/event bases, then enables command/event
/// processing and finally translation.  It intentionally does not enable the
/// Radeon PCI bus-master bit; that remains a later GPU-side promotion gate.
pub unsafe fn program_hardware_unit(mmio: u64, plan: AmdViHardwarePlan) -> Result<(), &'static str> {
    plan.validate()?;
    if mmio == 0 { return Err("AMD-Vi C6 mapped MMIO is null"); }
    let wr = |off:u64,val:u64| unsafe { ptr::write_volatile((mmio+off) as *mut u64,val) };
    let rd = |off:u64| unsafe { ptr::read_volatile((mmio+off) as *const u64) };

    // Never inherit a partially enabled engine.
    let inherited = rd(AMDVI_REG_CONTROL);
    if inherited & AMDVI_CONTROL_IOMMU_ENABLE != 0 {
        return Err("AMD-Vi C6 refuses to reprogram an already-enabled unit");
    }
    wr(AMDVI_REG_COMMAND_HEAD,0); wr(AMDVI_REG_COMMAND_TAIL,0);
    wr(AMDVI_REG_EVENT_HEAD,0); wr(AMDVI_REG_EVENT_TAIL,0);

    // Base encodings are kept page aligned; size/feature fields are validated
    // by the platform qualification layer before this primitive is promoted.
    wr(AMDVI_REG_DEVICE_TABLE_BASE, plan.device_table & AMDVI_ADDR_MASK);
    wr(AMDVI_REG_COMMAND_BUFFER_BASE, plan.command_buffer & AMDVI_ADDR_MASK);
    wr(AMDVI_REG_EVENT_LOG_BASE, plan.event_log & AMDVI_ADDR_MASK);

    let control = AMDVI_CONTROL_COMMAND_BUFFER_ENABLE | AMDVI_CONTROL_EVENT_LOG_ENABLE;
    wr(AMDVI_REG_CONTROL, control);
    wr(AMDVI_REG_CONTROL, control | AMDVI_CONTROL_IOMMU_ENABLE);
    let observed = rd(AMDVI_REG_CONTROL);
    if observed & AMDVI_CONTROL_IOMMU_ENABLE == 0 { return Err("AMD-Vi C6 translation enable did not latch"); }
    Ok(())
}

pub fn c6_register_self_test()->Result<(),&'static str>{
    if AMDVI_MMIO_BYTES < 0x2030 { return Err("AMD-Vi C6 MMIO span too small"); }
    if AMDVI_REG_DEVICE_TABLE_BASE != 0 || AMDVI_REG_CONTROL != 0x18 || AMDVI_REG_STATUS != 0x2020 { return Err("AMD-Vi C6 register map invariant failed"); }
    if AMDVI_CONTROL_IOMMU_ENABLE == 0 || AMDVI_CONTROL_COMMAND_BUFFER_ENABLE == 0 || AMDVI_CONTROL_EVENT_LOG_ENABLE == 0 { return Err("AMD-Vi C6 control bits invalid"); }
    Ok(())
}
