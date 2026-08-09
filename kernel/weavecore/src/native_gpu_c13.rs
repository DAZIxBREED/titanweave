//! K14.C13 physical Radeon read-proof qualification.
//!
//! C12 can perform the first bounded status reads when its prerequisites are
//! live. C13 does not add any destructive GPU capability. It turns those reads
//! into a durable qualification proof: exact PCI identity, inherited C12 read
//! evidence, all-ones/MMIO-absence rejection, bus-master recheck, and a compact
//! fingerprint suitable for serial-log comparison on bare metal.
//!
//! Navi48 remains fail-closed until trusted AMD IP-discovery data supplies its
//! GC/SDMA base records. C13 never substitutes guessed offsets.

use crate::{native_gpu_c9, native_gpu_c12, native_gpu_binding, pci, serial, sync::SpinLock};

pub const K14C13_ABI_VERSION: u32 = 1;
pub const RADEON_C13_MMIO_WRITES_ALLOWED: bool = false;
pub const RADEON_C13_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C13_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C13_BUS_MASTER_ALLOWED: bool = false;
pub const RADEON_C13_REQUIRED_READS: u8 = 3;

#[derive(Clone, Copy, Debug)]
pub struct C13State {
    pub amd_present: bool,
    pub profile_verified: bool,
    pub c12_live_read_proof: bool,
    pub read_values_sane: bool,
    pub bus_master_rechecked_off: bool,
    pub physical_proof_complete: bool,
    pub navi48_discovery_pending: bool,
    pub reads_inherited: u8,
    pub proof_fingerprint: u64,
    pub device_id: u16,
    pub revision: u8,
    pub write_path_fenced: bool,
    pub fallback_armed: bool,
}
impl C13State {
    pub const EMPTY: Self = Self {
        amd_present:false, profile_verified:false, c12_live_read_proof:false,
        read_values_sane:false, bus_master_rechecked_off:false,
        physical_proof_complete:false, navi48_discovery_pending:false,
        reads_inherited:0, proof_fingerprint:0, device_id:0, revision:0,
        write_path_fenced:true, fallback_armed:true,
    };
}
static STATE: SpinLock<C13State> = SpinLock::new(C13State::EMPTY);

fn fingerprint(device:u16, revision:u8, values:[u32;3])->u64 {
    // FNV-1a is only a compact evidence fingerprint here, not a security hash.
    let mut h=0xcbf29ce484222325u64;
    for b in device.to_le_bytes().into_iter()
        .chain([revision])
        .chain(values[0].to_le_bytes())
        .chain(values[1].to_le_bytes())
        .chain(values[2].to_le_bytes()) {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn self_test()->Result<(), &'static str> {
    if K14C13_ABI_VERSION != 1 || RADEON_C13_REQUIRED_READS != 3
        || RADEON_C13_MMIO_WRITES_ALLOWED || RADEON_C13_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C13_COMMAND_SUBMIT_ALLOWED || RADEON_C13_BUS_MASTER_ALLOWED {
        return Err("K14.C13 fail-closed constants invalid");
    }
    if fingerprint(0x73bf,0xc1,[1,2,3]) == 0 { return Err("K14.C13 proof fingerprint self-test failed"); }
    Ok(())
}

pub fn initialize()->Result<C13State,&'static str> {
    self_test()?;
    let c9=native_gpu_c9::state();
    let c12=native_gpu_c12::state();
    let mut s=C13State{
        amd_present:c9.amd_present, profile_verified:c9.profile_verified,
        reads_inherited:c12.live_reads_performed, device_id:c9.device_id,
        revision:c9.revision, ..C13State::EMPTY
    };

    serial::println(format_args!(
        "[C13PV] physical read-proof policy: require_exact_profile=true require_C12_reads={} reject_all_ones=true recheck_bus_master_off=true writes=false upload=false submit=false",
        RADEON_C13_REQUIRED_READS
    ));

    if !c9.amd_present {
        serial::println(format_args!(
            "[C13HW] physical Radeon qualification: present=false qemu_deferred=true proof=false fallback=true"
        ));
    } else if c9.profile == native_gpu_c9::ProfileId::Navi48Rx9070 && c12.live_reads_performed == 0 {
        s.navi48_discovery_pending=true;
        serial::println(format_args!(
            "[C13HW] physical Radeon qualification: present=true devid={:#06x} profile=Navi48 trusted_ip_discovery_required=true proof=false fallback=true",
            c9.device_id
        ));
    } else {
        s.c12_live_read_proof =
            c9.profile_verified && c9.pci_identity_consistent &&
            c12.live_read_gate_ready &&
            c12.live_reads_performed == RADEON_C13_REQUIRED_READS &&
            c12.grbm_status_valid && c12.chip_revision_valid && c12.sdma_status_valid;

        if s.c12_live_read_proof {
            let vals=[c12.grbm_status,c12.chip_revision,c12.sdma_status];
            // 0xffff_ffff is the canonical absent/unimplemented MMIO response.
            // Do not reject zero: status registers can legitimately be idle/zero.
            s.read_values_sane = !vals.iter().all(|v| *v == 0xffff_ffff);
            let b=native_gpu_binding::state();
            let command=pci::read_u16(b.selected_bus,b.selected_device,b.selected_function,0x04);
            s.bus_master_rechecked_off = command & (1<<2) == 0;
            if !s.bus_master_rechecked_off {
                return Err("K14.C13 Radeon bus mastering became enabled during read-only qualification");
            }
            if !s.read_values_sane {
                return Err("K14.C13 Radeon MMIO proof returned only all-ones values");
            }
            s.proof_fingerprint=fingerprint(c9.device_id,c9.revision,vals);
            s.physical_proof_complete=true;
            serial::println(format_args!(
                "[C13SN] Radeon status sanity: GRBM={:#010x} CHIPREV={:#010x} SDMA={:#010x} all_ones=false bus_master=false fingerprint={:#018x}",
                vals[0],vals[1],vals[2],s.proof_fingerprint
            ));
            serial::println(format_args!(
                "[C13HW] physical Radeon qualification: present=true devid={:#06x} revision={:#04x} reads={} proof=true writes=false bus_master=false fallback=true",
                c9.device_id,c9.revision,s.reads_inherited
            ));
        } else {
            serial::println(format_args!(
                "[C13HW] physical Radeon qualification: present=true devid={:#06x} c12_read_proof=false reads={} proof=false fallback=true",
                c9.device_id,c12.live_reads_performed
            ));
        }
    }

    if s.physical_proof_complete && (!s.c12_live_read_proof || !s.read_values_sane || !s.bus_master_rechecked_off) {
        return Err("K14.C13 physical proof promoted without all safety gates");
    }
    if !s.write_path_fenced || RADEON_C13_MMIO_WRITES_ALLOWED || RADEON_C13_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C13_COMMAND_SUBMIT_ALLOWED || RADEON_C13_BUS_MASTER_ALLOWED {
        return Err("K14.C13 destructive Radeon capability promoted early");
    }

    serial::println(format_args!(
        "[C13RD] K14.C13 physical read-proof ready: amd_present={} profile_verified={} c12_proof={} sane={} bus_master_off={} physical_proof={} navi48_discovery_pending={} reads={} fingerprint={:#018x} writes=false fallback=true",
        s.amd_present,s.profile_verified,s.c12_live_read_proof,s.read_values_sane,
        s.bus_master_rechecked_off,s.physical_proof_complete,s.navi48_discovery_pending,
        s.reads_inherited,s.proof_fingerprint
    ));
    *STATE.lock()=s;
    Ok(s)
}
pub fn state()->C13State{*STATE.lock()}
pub fn packed_status()->u64{
    let s=state();
    let mut v=(u64::from(s.device_id)<<32)|(u64::from(s.revision)<<24)|(u64::from(s.reads_inherited)<<16);
    for (bit,on) in [s.amd_present,s.profile_verified,s.c12_live_read_proof,s.read_values_sane,
        s.bus_master_rechecked_off,s.physical_proof_complete,s.navi48_discovery_pending,
        s.write_path_fenced,s.fallback_armed].into_iter().enumerate(){ if on { v|=1u64<<bit; } }
    v
}
