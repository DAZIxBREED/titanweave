//! K14.C19 bounded physical AMD discovery-locator acquisition.
//!
//! C18 proved the AMD discovery checksum rules and TMR contract. C19 opens the
//! next narrow physical boundary: read-only acquisition of the discovery TMR
//! locator from source-backed Radeon registers. It does NOT read the discovery
//! payload yet. The snapshot itself remains fenced until a bounded CPU-visible
//! memory/VRAM path is independently proven.
//!
//! Source-backed register contract imported from upstream AMDGPU:
//!   mmRCC_CONFIG_MEMSIZE   = 0x0de3
//!   mmDRIVER_SCRATCH_0     = 0x0094
//!   mmDRIVER_SCRATCH_1     = 0x0095
//!   mmDRIVER_SCRATCH_2     = 0x0096
//! These are direct MMIO register indices and are converted to byte offsets by
//! multiplying by four before mapping BAR5 read-only.

use crate::{
    memory::FrameAllocator,
    native_gpu_binding,
    native_gpu_c6,
    native_gpu_c9,
    native_gpu_c12,
    native_gpu_c18,
    paging,
    pci,
    serial,
    sync::SpinLock,
};

pub const K14C19_ABI_VERSION: u32 = 1;
pub const RADEON_C19_MMIO_BAR_INDEX: u8 = 5;
pub const RADEON_C19_PAGE_BYTES: u64 = 4096;
pub const RADEON_C19_MAX_LOCATOR_READS: u8 = 4;
pub const RADEON_C19_MAX_DISCOVERY_BYTES: u32 = 64 * 1024;

pub const RADEON_C19_LOCATOR_READS_ALLOWED: bool = true;
pub const RADEON_C19_LIVE_TMR_PAYLOAD_READ_ALLOWED: bool = false;
pub const RADEON_C19_LIVE_VRAM_READ_ALLOWED: bool = false;
pub const RADEON_C19_MMIO_WRITE_ALLOWED: bool = false;
pub const RADEON_C19_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C19_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C19_BUS_MASTER_ALLOWED: bool = false;

#[derive(Clone, Copy, Debug)]
pub struct C19State {
    pub amd_present: bool,
    pub navi48: bool,
    pub c18_ready: bool,
    pub exact_domain_live: bool,
    pub bar5_ready: bool,
    pub locator_reads_performed: u8,
    pub rcc_memsize_valid: bool,
    pub scratch_locator_valid: bool,
    pub locator_acquired: bool,
    pub tmr_in_vram: bool,
    pub tmr_offset: u64,
    pub tmr_size: u32,
    pub payload_read_promoted: bool,
    pub live_snapshot_acquired: bool,
    pub live_snapshot_verified: bool,
    pub bus_master_rechecked_off: bool,
    pub fallback_armed: bool,
    pub device_id: u16,
    pub revision: u8,
}
impl C19State {
    pub const EMPTY: Self = Self {
        amd_present:false,navi48:false,c18_ready:false,exact_domain_live:false,
        bar5_ready:false,locator_reads_performed:0,rcc_memsize_valid:false,
        scratch_locator_valid:false,locator_acquired:false,tmr_in_vram:false,
        tmr_offset:0,tmr_size:0,payload_read_promoted:false,
        live_snapshot_acquired:false,live_snapshot_verified:false,
        bus_master_rechecked_off:false,fallback_armed:true,device_id:0,revision:0,
    };
}
static STATE: SpinLock<C19State> = SpinLock::new(C19State::EMPTY);

fn register_mmio_bar() -> Option<u64> {
    let b = native_gpu_binding::state();
    pci::memory_bar_base(
        crate::pci::PciFunction {
            bus:b.selected_bus,device:b.selected_device,function:b.selected_function,
            vendor_id:0,device_id:0,class_code:0,subclass:0,programming_interface:0,
            revision:0,header_type:0,
        },
        RADEON_C19_MMIO_BAR_INDEX,
    )
}

unsafe fn read_ro_reg(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
    bar_phys: u64,
    reg_dword: u32,
) -> Result<u32, &'static str> {
    let byte_offset = u64::from(reg_dword)
        .checked_mul(4).ok_or("K14.C19 register byte-offset overflow")?;
    if byte_offset > 0x00ff_ffff {
        return Err("K14.C19 locator register outside bounded direct-MMIO window");
    }
    let page_offset = byte_offset & !(RADEON_C19_PAGE_BYTES - 1);
    let in_page = byte_offset & (RADEON_C19_PAGE_BYTES - 1);
    if in_page + 4 > RADEON_C19_PAGE_BYTES || in_page & 3 != 0 {
        return Err("K14.C19 unaligned locator register");
    }
    let phys = bar_phys.checked_add(page_offset).ok_or("K14.C19 MMIO physical overflow")?;
    let virt = paging::map_kernel_mmio_readonly(
        allocator, kernel_cr3, phys, RADEON_C19_PAGE_BYTES
    )?;
    Ok(unsafe { core::ptr::read_volatile((virt + in_page) as *const u32) })
}

fn derive_locator(vram_size_mb:u32, scratch0:u32, scratch1:u32, scratch2:u32)
    -> Result<(bool,u64,u32,bool), &'static str>
{
    if [vram_size_mb,scratch0,scratch1,scratch2].iter().any(|v| *v == u32::MAX) {
        return Err("K14.C19 invalid all-ones Radeon locator register response");
    }
    if scratch2 != 0 {
        let size = scratch2;
        if size < 64 || size > RADEON_C19_MAX_DISCOVERY_BYTES {
            return Err("K14.C19 scratch discovery size outside bounded policy");
        }
        let offset = (u64::from(scratch1) << 32) | u64::from(scratch0);
        if offset == 0 { return Err("K14.C19 scratch discovery offset is zero"); }
        return Ok((vram_size_mb != 0, offset, size, true));
    }
    if vram_size_mb != 0 {
        let total = u64::from(vram_size_mb)
            .checked_shl(20).ok_or("K14.C19 VRAM-size shift overflow")?;
        let offset = total.checked_sub(native_gpu_c18::AMD_DISCOVERY_TMR_OFFSET)
            .ok_or("K14.C19 default discovery TMR underflow")?;
        return Ok((true, offset, native_gpu_c18::AMD_DISCOVERY_TMR_SIZE, false));
    }
    Err("K14.C19 system-memory TMR requires later ACPI locator path")
}

fn self_test() -> Result<(), &'static str> {
    if K14C19_ABI_VERSION != 1
        || RADEON_C19_MMIO_BAR_INDEX != native_gpu_c12::RADEON_C12_MMIO_BAR_INDEX
        || RADEON_C19_MAX_LOCATOR_READS != 4
        || !RADEON_C19_LOCATOR_READS_ALLOWED
        || RADEON_C19_LIVE_TMR_PAYLOAD_READ_ALLOWED
        || RADEON_C19_LIVE_VRAM_READ_ALLOWED
        || RADEON_C19_MMIO_WRITE_ALLOWED
        || RADEON_C19_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C19_COMMAND_SUBMIT_ALLOWED
        || RADEON_C19_BUS_MASTER_ALLOWED
    { return Err("K14.C19 fail-closed constants invalid"); }

    let (in_vram,off,size,scratch) = derive_locator(16*1024,0,0,0)?;
    if !in_vram || scratch || size != native_gpu_c18::AMD_DISCOVERY_TMR_SIZE
        || off != ((16u64*1024)<<20) - native_gpu_c18::AMD_DISCOVERY_TMR_OFFSET
    { return Err("K14.C19 default VRAM locator self-test failed"); }

    let (in_vram2,off2,size2,scratch2) =
        derive_locator(16*1024,0x1234_5000,0x0000_0001,10*1024)?;
    if !in_vram2 || !scratch2 || off2 != 0x0000_0001_1234_5000 || size2 != 10*1024 {
        return Err("K14.C19 scratch locator self-test failed");
    }
    Ok(())
}

pub fn initialize(
    allocator:&mut FrameAllocator<'_>,
    kernel_cr3:u64,
) -> Result<C19State,&'static str> {
    self_test()?;
    let c18 = native_gpu_c18::state();
    let c9 = native_gpu_c9::state();
    let c6 = native_gpu_c6::state();
    let binding = native_gpu_binding::state();
    let mut s = C19State {
        amd_present:c9.amd_present,
        navi48:c9.profile == native_gpu_c9::ProfileId::Navi48Rx9070,
        c18_ready:c18.checksum_engine_ready && c18.tmr_contract_imported
            && c18.synthetic_checksum_selftest_passed,
        exact_domain_live:c6.persistent_domain_live,
        device_id:c9.device_id,
        revision:c9.revision,
        ..C19State::EMPTY
    };

    serial::println(format_args!(
        "[C19PG] physical discovery-locator policy: BAR5={} reads={} regs={:#x}/{:#x}/{:#x}/{:#x} max_snapshot={} payload_read=false VRAM_read=false writes=false firmware=false submit=false bus_master_enable=false",
        RADEON_C19_MMIO_BAR_INDEX,RADEON_C19_MAX_LOCATOR_READS,
        native_gpu_c18::AMD_DISCOVERY_MM_RCC_CONFIG_MEMSIZE,
        native_gpu_c18::AMD_DISCOVERY_MM_DRIVER_SCRATCH_0,
        native_gpu_c18::AMD_DISCOVERY_MM_DRIVER_SCRATCH_1,
        native_gpu_c18::AMD_DISCOVERY_MM_DRIVER_SCRATCH_2,
        RADEON_C19_MAX_DISCOVERY_BYTES
    ));

    if !s.amd_present {
        serial::println(format_args!(
            "[C19HW] physical AMD discovery locator: present=false qemu_deferred=true locator=false payload=false verified=false fallback=true"
        ));
    } else {
        if !s.c18_ready || !s.exact_domain_live {
            return Err("K14.C19 physical locator attempted before C18/domain prerequisites");
        }
        let command = pci::read_u16(
            binding.selected_bus,binding.selected_device,binding.selected_function,0x04
        );
        s.bus_master_rechecked_off = command & (1<<2) == 0;
        if !s.bus_master_rechecked_off {
            return Err("K14.C19 Radeon bus mastering unexpectedly enabled");
        }
        let bar = register_mmio_bar().ok_or("K14.C19 Radeon BAR5 unavailable")?;
        s.bar5_ready = true;
        let rcc = unsafe { read_ro_reg(allocator,kernel_cr3,bar,native_gpu_c18::AMD_DISCOVERY_MM_RCC_CONFIG_MEMSIZE)? };
        let lo  = unsafe { read_ro_reg(allocator,kernel_cr3,bar,native_gpu_c18::AMD_DISCOVERY_MM_DRIVER_SCRATCH_0)? };
        let hi  = unsafe { read_ro_reg(allocator,kernel_cr3,bar,native_gpu_c18::AMD_DISCOVERY_MM_DRIVER_SCRATCH_1)? };
        let sz  = unsafe { read_ro_reg(allocator,kernel_cr3,bar,native_gpu_c18::AMD_DISCOVERY_MM_DRIVER_SCRATCH_2)? };
        s.locator_reads_performed = 4;
        s.rcc_memsize_valid = rcc != u32::MAX;
        let (in_vram,off,size,scratch) = derive_locator(rcc,lo,hi,sz)?;
        s.tmr_in_vram = in_vram;
        s.tmr_offset = off;
        s.tmr_size = size;
        s.scratch_locator_valid = scratch;
        s.locator_acquired = true;
        serial::println(format_args!(
            "[C19HW] physical AMD discovery locator: present=true navi48={} devid={:#06x} BAR5=true reads=4 vram_mb={} scratch_override={} tmr_in_vram={} tmr_offset={:#018x} tmr_size={} locator=true payload=false verified=false bus_master=false fallback=true",
            s.navi48,s.device_id,rcc,scratch,s.tmr_in_vram,s.tmr_offset,s.tmr_size
        ));
    }

    if s.locator_reads_performed > RADEON_C19_MAX_LOCATOR_READS {
        return Err("K14.C19 locator read budget exceeded");
    }
    if s.live_snapshot_acquired || s.live_snapshot_verified || s.payload_read_promoted {
        return Err("K14.C19 discovery payload promoted early");
    }
    if RADEON_C19_MMIO_WRITE_ALLOWED || RADEON_C19_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C19_COMMAND_SUBMIT_ALLOWED || RADEON_C19_BUS_MASTER_ALLOWED {
        return Err("K14.C19 destructive capability promoted early");
    }

    serial::println(format_args!(
        "[C19RD] K14.C19 physical-locator gate ready: amd_present={} navi48={} C18_ready={} domain_live={} BAR5={} reads={} locator={} tmr_in_vram={} tmr_offset={:#x} tmr_size={} payload=false snapshot=false verified=false bus_master_off={} fallback=true",
        s.amd_present,s.navi48,s.c18_ready,s.exact_domain_live,s.bar5_ready,
        s.locator_reads_performed,s.locator_acquired,s.tmr_in_vram,s.tmr_offset,
        s.tmr_size,s.bus_master_rechecked_off
    ));
    *STATE.lock() = s;
    Ok(s)
}

pub fn state()->C19State { *STATE.lock() }
pub fn packed_status()->u64 {
    let s=state();
    let mut v=(u64::from(s.device_id)<<32)|(u64::from(s.revision)<<24);
    for (bit,on) in [
        s.amd_present,s.navi48,s.c18_ready,s.exact_domain_live,s.bar5_ready,
        s.locator_acquired,s.tmr_in_vram,s.bus_master_rechecked_off,
        s.live_snapshot_acquired,s.live_snapshot_verified,s.fallback_armed
    ].into_iter().enumerate() {
        if on { v |= 1u64<<bit; }
    }
    v
}
