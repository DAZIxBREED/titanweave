pub mod apic;
pub mod gdt;
pub mod idt;
pub mod pit;
pub mod port;
pub mod smp;

use core::arch::asm;

pub const INTERRUPT_FLAG: u64 = 1 << 9;

#[derive(Clone, Copy, Debug)]
pub struct ExecutionState {
    pub cr0_before: u64,
    pub cr0_after: u64,
    pub cr4_before: u64,
    pub cr4_after: u64,
}

/// Establish the architectural execution state WeaveCore expects instead of
/// inheriting optional firmware state. In particular, K11 does not yet manage
/// CET shadow stacks per kernel task, so supervisor CET must be disabled before
/// switching RSP between independently-created task stacks. The x87/SSE control
/// bits are also normalized so a stale CR0.TS/EM cannot turn the first generated
/// arithmetic instruction into an unexpected #NM.
pub fn initialize_execution_state() -> ExecutionState {
    const CR0_MP: u64 = 1 << 1;
    const CR0_EM: u64 = 1 << 2;
    const CR0_TS: u64 = 1 << 3;
    const CR0_NE: u64 = 1 << 5;
    const CR4_OSFXSR: u64 = 1 << 9;
    const CR4_OSXMMEXCPT: u64 = 1 << 10;
    const CR4_CET: u64 = 1 << 23;

    let cr0_before: u64;
    let cr4_before: u64;
    unsafe {
        asm!("mov {}, cr0", out(reg) cr0_before, options(nomem, nostack, preserves_flags));
        asm!("mov {}, cr4", out(reg) cr4_before, options(nomem, nostack, preserves_flags));
    }

    let cr0_after = (cr0_before | CR0_MP | CR0_NE) & !(CR0_EM | CR0_TS);
    let cr4_after = (cr4_before | CR4_OSFXSR | CR4_OSXMMEXCPT) & !CR4_CET;

    unsafe {
        if cr0_after != cr0_before {
            asm!("mov cr0, {}", in(reg) cr0_after, options(nomem, nostack, preserves_flags));
        }
        if cr4_after != cr4_before {
            asm!("mov cr4, {}", in(reg) cr4_after, options(nomem, nostack, preserves_flags));
        }
        asm!("fninit", options(nomem, nostack, preserves_flags));
    }

    ExecutionState { cr0_before, cr0_after, cr4_before, cr4_after }
}

pub fn halt_forever() -> ! {
    loop {
        unsafe { asm!("cli", "hlt", options(nomem, nostack)) };
    }
}

pub fn halt_once() {
    unsafe { asm!("hlt", options(nomem, nostack)) };
}

pub fn pause() {
    unsafe { asm!("pause", options(nomem, nostack, preserves_flags)) };
}

pub fn disable_interrupts() {
    unsafe { asm!("cli", options(nostack)) };
}

pub fn enable_interrupts() {
    unsafe { asm!("sti", options(nostack)) };
}

#[must_use]
pub fn read_rflags() -> u64 {
    let flags: u64;
    unsafe {
        asm!(
            "pushfq",
            "pop {}",
            out(reg) flags,
            options(nomem, preserves_flags)
        );
    }
    flags
}

#[must_use]
pub fn interrupts_enabled() -> bool {
    read_rflags() & INTERRUPT_FLAG != 0
}

/// Disable local interrupts and return the previous RFLAGS value.
#[must_use]
pub fn save_and_disable_interrupts() -> u64 {
    let flags = read_rflags();
    disable_interrupts();
    flags
}

/// Restore only the interrupt-enable state captured by
/// [`save_and_disable_interrupts`].
pub fn restore_interrupts(flags: u64) {
    if flags & INTERRUPT_FLAG != 0 {
        enable_interrupts();
    } else {
        disable_interrupts();
    }
}

pub fn read_msr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack)
        );
    }
    ((high as u64) << 32) | low as u64
}

pub fn write_msr(msr: u32, value: u64) {
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") value as u32,
            in("edx") (value >> 32) as u32,
            options(nomem, nostack)
        );
    }
}

/// Enable the x86-64 execute-disable page-table bit through EFER.NXE.
pub fn enable_nx() {
    const IA32_EFER: u32 = 0xc000_0080;
    const EFER_NXE: u64 = 1 << 11;
    let value = read_msr(IA32_EFER);
    if value & EFER_NXE == 0 {
        write_msr(IA32_EFER, value | EFER_NXE);
    }
}
