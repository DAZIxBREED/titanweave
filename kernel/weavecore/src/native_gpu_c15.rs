//! K14.C15 first controlled Radeon write transaction.
//!
//! C14 proves that write-side prerequisites are complete but deliberately does
//! not perform a write. C15 introduces the first real write transaction while
//! still refusing Radeon MMIO writes. The transaction is a PCI Command-word
//! identity write on the selected Radeon: Titanweave reads the 16-bit Command
//! word, requires bus mastering to be clear, writes the exact same 16-bit value
//! back through a width-correct PCI config access, reads it back, and proves
//! both equality and continued bus-master disablement.
//!
//! Why this stage exists: it qualifies write serialization, readback, rollback,
//! and evidence logging without changing GPU-visible state. Writing the 16-bit
//! Command word avoids rewriting the adjacent PCI Status word, whose semantics
//! can include write-one-to-clear bits. Radeon MMIO writes, firmware upload,
//! command submission, and bus-master enable remain hard-fenced.

use crate::{
    native_gpu_c9,
    native_gpu_c14,
    native_gpu_binding,
    pci,
    serial,
    sync::SpinLock,
};

pub const K14C15_ABI_VERSION: u32 = 1;
pub const RADEON_C15_PCI_IDENTITY_WRITE_ALLOWED: bool = true;
pub const RADEON_C15_MMIO_WRITES_ALLOWED: bool = false;
pub const RADEON_C15_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C15_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C15_BUS_MASTER_ALLOWED: bool = false;
pub const RADEON_C15_MAX_WRITE_ATTEMPTS: u8 = 2; // identity write + one rollback retry

#[derive(Clone, Copy, Debug)]
pub struct C15State {
    pub amd_present: bool,
    pub c14_prerequisites_complete: bool,
    pub transaction_eligible: bool,
    pub identity_write_attempted: bool,
    pub identity_write_verified: bool,
    pub rollback_attempted: bool,
    pub rollback_verified: bool,
    pub bus_master_before_off: bool,
    pub bus_master_after_off: bool,
    pub command_before: u16,
    pub command_after: u16,
    pub write_attempts: u8,
    pub transaction_fingerprint: u64,
    pub device_id: u16,
    pub revision: u8,
    pub mmio_write_path_fenced: bool,
    pub fallback_armed: bool,
}

impl C15State {
    pub const EMPTY: Self = Self {
        amd_present:false,
        c14_prerequisites_complete:false,
        transaction_eligible:false,
        identity_write_attempted:false,
        identity_write_verified:false,
        rollback_attempted:false,
        rollback_verified:false,
        bus_master_before_off:false,
        bus_master_after_off:false,
        command_before:0,
        command_after:0,
        write_attempts:0,
        transaction_fingerprint:0,
        device_id:0,
        revision:0,
        mmio_write_path_fenced:true,
        fallback_armed:true,
    };
}

static STATE: SpinLock<C15State> = SpinLock::new(C15State::EMPTY);

fn fingerprint(device:u16, revision:u8, before:u16, after:u16, attempts:u8)->u64 {
    let mut h=0xcbf29ce484222325u64;
    for b in device.to_le_bytes().into_iter()
        .chain([revision])
        .chain(before.to_le_bytes())
        .chain(after.to_le_bytes())
        .chain([attempts]) {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn self_test()->Result<(), &'static str> {
    if K14C15_ABI_VERSION != 1
        || !RADEON_C15_PCI_IDENTITY_WRITE_ALLOWED
        || RADEON_C15_MMIO_WRITES_ALLOWED
        || RADEON_C15_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C15_COMMAND_SUBMIT_ALLOWED
        || RADEON_C15_BUS_MASTER_ALLOWED
        || RADEON_C15_MAX_WRITE_ATTEMPTS != 2
    {
        return Err("K14.C15 fail-closed write-transaction constants invalid");
    }
    if fingerprint(0x73bf,0xc1,0x0002,0x0002,1)==0 {
        return Err("K14.C15 transaction fingerprint self-test failed");
    }
    Ok(())
}

pub fn initialize()->Result<C15State,&'static str> {
    self_test()?;
    let c9=native_gpu_c9::state();
    let c14=native_gpu_c14::state();
    let mut s=C15State{
        amd_present:c9.amd_present,
        c14_prerequisites_complete:c14.write_prerequisites_complete,
        device_id:c9.device_id,
        revision:c9.revision,
        ..C15State::EMPTY
    };

    serial::println(format_args!(
        "[C15TX] controlled-write policy: target=PCI_COMMAND width=16 identity_write=true require_C14=true require_bus_master_off=true readback=true rollback=true MMIO_writes=false firmware_upload=false submit=false bus_master_enable=false"
    ));

    if !s.amd_present {
        serial::println(format_args!(
            "[C15HW] controlled Radeon write transaction: present=false qemu_deferred=true attempted=false verified=false MMIO_writes=false fallback=true"
        ));
    } else if !c14.write_prerequisites_complete {
        serial::println(format_args!(
            "[C15HW] controlled Radeon write transaction: present=true devid={:#06x} C14_prerequisites=false attempted=false verified=false reason=write_readiness_not_proven fallback=true",
            c9.device_id
        ));
    } else {
        let b=native_gpu_binding::state();
        s.transaction_eligible=true;
        s.command_before=pci::read_u16(b.selected_bus,b.selected_device,b.selected_function,0x04);
        s.bus_master_before_off=s.command_before&(1<<2)==0;
        if !s.bus_master_before_off {
            return Err("K14.C15 bus mastering enabled before identity-write transaction");
        }

        // First real write-side transaction: write back exactly the current
        // 16-bit Command value. No bit is intentionally changed.
        pci::write_u16(b.selected_bus,b.selected_device,b.selected_function,0x04,s.command_before);
        s.write_attempts=1;
        s.identity_write_attempted=true;
        s.command_after=pci::read_u16(b.selected_bus,b.selected_device,b.selected_function,0x04);
        s.bus_master_after_off=s.command_after&(1<<2)==0;
        s.identity_write_verified=s.command_after==s.command_before && s.bus_master_after_off;

        if !s.identity_write_verified {
            // Bounded rollback: restore the original 16-bit Command word once.
            s.rollback_attempted=true;
            pci::write_u16(b.selected_bus,b.selected_device,b.selected_function,0x04,s.command_before);
            s.write_attempts=2;
            let restored=pci::read_u16(b.selected_bus,b.selected_device,b.selected_function,0x04);
            s.rollback_verified=restored==s.command_before && restored&(1<<2)==0;
            s.command_after=restored;
            s.bus_master_after_off=restored&(1<<2)==0;
            if !s.rollback_verified {
                return Err("K14.C15 PCI Command identity-write readback and rollback both failed");
            }
            return Err("K14.C15 PCI Command identity-write required rollback; transaction not qualified");
        }

        s.transaction_fingerprint=fingerprint(
            s.device_id,s.revision,s.command_before,s.command_after,s.write_attempts
        );
        serial::println(format_args!(
            "[C15RB] controlled write readback: command_before={:#06x} command_after={:#06x} equal=true bus_master_before=false bus_master_after=false attempts={} rollback=false fingerprint={:#018x}",
            s.command_before,s.command_after,s.write_attempts,s.transaction_fingerprint
        ));
        serial::println(format_args!(
            "[C15HW] controlled Radeon write transaction: present=true devid={:#06x} revision={:#04x} eligible=true attempted=true verified=true target=PCI_COMMAND identity=true MMIO_writes=false bus_master=false fallback=true",
            s.device_id,s.revision
        ));
    }

    if s.identity_write_verified && (!s.transaction_eligible || !s.identity_write_attempted || !s.bus_master_before_off || !s.bus_master_after_off) {
        return Err("K14.C15 write transaction qualified without all safety gates");
    }
    if s.write_attempts>RADEON_C15_MAX_WRITE_ATTEMPTS {
        return Err("K14.C15 exceeded bounded write-attempt count");
    }
    if !s.mmio_write_path_fenced || RADEON_C15_MMIO_WRITES_ALLOWED || RADEON_C15_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C15_COMMAND_SUBMIT_ALLOWED || RADEON_C15_BUS_MASTER_ALLOWED {
        return Err("K14.C15 destructive Radeon capability promoted early");
    }

    serial::println(format_args!(
        "[C15RD] K14.C15 controlled write transaction ready: amd_present={} C14_prerequisites={} eligible={} attempted={} verified={} rollback_attempted={} rollback_verified={} bus_master_before_off={} bus_master_after_off={} attempts={} fingerprint={:#018x} MMIO_writes=false upload=false submit=false bus_master=false fallback=true",
        s.amd_present,s.c14_prerequisites_complete,s.transaction_eligible,s.identity_write_attempted,
        s.identity_write_verified,s.rollback_attempted,s.rollback_verified,s.bus_master_before_off,
        s.bus_master_after_off,s.write_attempts,s.transaction_fingerprint
    ));
    *STATE.lock()=s;
    Ok(s)
}

pub fn state()->C15State{*STATE.lock()}
pub fn packed_status()->u64{
    let s=state();
    let mut v=(u64::from(s.device_id)<<32)|(u64::from(s.revision)<<24)|(u64::from(s.write_attempts)<<16);
    for (bit,on) in [s.amd_present,s.c14_prerequisites_complete,s.transaction_eligible,s.identity_write_attempted,
        s.identity_write_verified,s.rollback_attempted,s.rollback_verified,s.bus_master_before_off,
        s.bus_master_after_off,s.mmio_write_path_fenced,s.fallback_armed].into_iter().enumerate(){ if on {v|=1u64<<bit;} }
    v
}
