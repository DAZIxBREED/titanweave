use crate::arch::x86_64::gdt::{KERNEL_CODE_SELECTOR, KERNEL_DATA_SELECTOR};
use crate::arch::x86_64::halt_forever;
use crate::{abi, process, serial};
use core::arch::{asm, global_asm};
use core::mem::size_of;

pub const TIMER_VECTOR: u8 = 0x40;
pub const RESCHEDULE_VECTOR: u8 = 0x41;
pub const SPURIOUS_VECTOR: u8 = 0xff;
pub const FIRST_DEVICE_VECTOR: u8 = 0x50;
pub const LAST_DEVICE_VECTOR: u8 = 0xdf;
const DEVICE_VECTOR_COUNT: usize = (LAST_DEVICE_VECTOR - FIRST_DEVICE_VECTOR + 1) as usize;

const DOUBLE_FAULT_IST: u8 = 1;
const NMI_IST: u8 = 2;
const MACHINE_CHECK_IST: u8 = 3;
// Dedicated diagnostic/fatal-fault stack. If the current task stack itself is
// bad, #PF/#GP/#SS/#TS/#NP/#NM/#CP can still report the original exception.
const FATAL_FAULT_IST: u8 = 4;

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    attributes: u8,
    offset_middle: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const MISSING: Self = Self {
        offset_low: 0,
        selector: 0,
        ist: 0,
        attributes: 0,
        offset_middle: 0,
        offset_high: 0,
        reserved: 0,
    };

    fn interrupt_gate(handler: u64, ist: u8) -> Self {
        Self::gate(handler, ist, 0x8e)
    }

    fn user_interrupt_gate(handler: u64) -> Self {
        // Present, DPL=3, 64-bit interrupt gate.
        Self::gate(handler, 0, 0xee)
    }

    fn gate(handler: u64, ist: u8, attributes: u8) -> Self {
        Self {
            offset_low: handler as u16,
            selector: KERNEL_CODE_SELECTOR,
            ist: ist & 0x07,
            attributes,
            offset_middle: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32,
            reserved: 0,
        }
    }
}

#[repr(C, packed)]
struct DescriptorTablePointer {
    limit: u16,
    base: u64,
}

#[repr(align(16))]
struct Idt([IdtEntry; 256]);

static mut IDT: Idt = Idt([IdtEntry::MISSING; 256]);

unsafe extern "C" {
    fn weave_isr_nmi();
    fn weave_isr_breakpoint();
    fn weave_isr_invalid_opcode();
    fn weave_isr_device_not_available();
    fn weave_isr_double_fault();
    fn weave_isr_invalid_tss();
    fn weave_isr_segment_not_present();
    fn weave_isr_stack_segment();
    fn weave_isr_general_protection();
    fn weave_isr_page_fault();
    fn weave_isr_machine_check();
    fn weave_isr_control_protection();
    fn weave_isr_timer();
    fn weave_isr_reschedule();
    fn weave_isr_syscall();
    fn weave_isr_spurious();
    static weave_device_isr_table: [usize; DEVICE_VECTOR_COUNT];
}

pub fn initialize() {
    unsafe {
        let idt = &raw mut IDT;
        (*idt).0[2] = IdtEntry::interrupt_gate(weave_isr_nmi as usize as u64, NMI_IST);
        (*idt).0[3] = IdtEntry::interrupt_gate(weave_isr_breakpoint as usize as u64, 0);
        (*idt).0[6] = IdtEntry::interrupt_gate(weave_isr_invalid_opcode as usize as u64, FATAL_FAULT_IST);
        (*idt).0[7] =
            IdtEntry::interrupt_gate(weave_isr_device_not_available as usize as u64, FATAL_FAULT_IST);
        (*idt).0[8] =
            IdtEntry::interrupt_gate(weave_isr_double_fault as usize as u64, DOUBLE_FAULT_IST);
        // These three faults all carry hardware error codes and are especially
        // important while bringing up task/context switching. Leaving them
        // absent can hide the original failure behind a double fault.
        (*idt).0[10] = IdtEntry::interrupt_gate(weave_isr_invalid_tss as usize as u64, FATAL_FAULT_IST);
        (*idt).0[11] = IdtEntry::interrupt_gate(weave_isr_segment_not_present as usize as u64, FATAL_FAULT_IST);
        (*idt).0[12] = IdtEntry::interrupt_gate(weave_isr_stack_segment as usize as u64, FATAL_FAULT_IST);
        (*idt).0[13] =
            IdtEntry::interrupt_gate(weave_isr_general_protection as usize as u64, FATAL_FAULT_IST);
        (*idt).0[14] = IdtEntry::interrupt_gate(weave_isr_page_fault as usize as u64, FATAL_FAULT_IST);
        (*idt).0[18] = IdtEntry::interrupt_gate(
            weave_isr_machine_check as usize as u64,
            MACHINE_CHECK_IST,
        );
        (*idt).0[21] =
            IdtEntry::interrupt_gate(weave_isr_control_protection as usize as u64, FATAL_FAULT_IST);
        (*idt).0[TIMER_VECTOR as usize] =
            IdtEntry::interrupt_gate(weave_isr_timer as usize as u64, 0);
        (*idt).0[RESCHEDULE_VECTOR as usize] =
            IdtEntry::interrupt_gate(weave_isr_reschedule as usize as u64, 0);
        (*idt).0[SPURIOUS_VECTOR as usize] =
            IdtEntry::interrupt_gate(weave_isr_spurious as usize as u64, 0);
        for index in 0..DEVICE_VECTOR_COUNT {
            let vector = FIRST_DEVICE_VECTOR as usize + index;
            // 0x80 is the ring-3 software syscall ABI. Device vectors span
            // across it, so never replace the DPL=3 syscall gate with a
            // DPL=0 hardware-interrupt gate.
            if vector == abi::SYSCALL_VECTOR as usize {
                continue;
            }
            (*idt).0[vector] = IdtEntry::interrupt_gate(weave_device_isr_table[index] as u64, 0);
        }
        // Install this after the device range as a second line of defense: the
        // final owner of vector 0x80 must always be the userspace syscall gate.
        (*idt).0[abi::SYSCALL_VECTOR as usize] =
            IdtEntry::user_interrupt_gate(weave_isr_syscall as usize as u64);
    }
    load();
}

pub fn load() {
    unsafe {
        let idt = &raw const IDT;
        let pointer = DescriptorTablePointer {
            limit: (size_of::<Idt>() - 1) as u16,
            base: idt as u64,
        };
        asm!("lidt [{}]", in(reg) &pointer, options(readonly, nostack));
    }
}

#[repr(C)]
pub struct InterruptFrame {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub vector: u64,
    pub error_code: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    // In 64-bit mode the hardware interrupt frame includes SS:RSP and IRETQ
    // restores them even for a return to CPL0. Keep these fields in the common
    // frame so synthetic scheduler frames exactly match the hardware layout.
    pub rsp: u64,
    pub ss: u64,
}

const _: [(); 176] = [(); size_of::<InterruptFrame>()];

impl InterruptFrame {
    pub const fn for_kernel_task(entry: u64, stack_pointer: u64) -> Self {
        Self {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rbp: 0,
            rdi: 0,
            rsi: 0,
            rdx: 0,
            rcx: 0,
            rbx: 0,
            rax: 0,
            vector: RESCHEDULE_VECTOR as u64,
            error_code: 0,
            rip: entry,
            cs: KERNEL_CODE_SELECTOR as u64,
            rflags: 0x202,
            rsp: stack_pointer,
            ss: KERNEL_DATA_SELECTOR as u64,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn weave_exception_dispatch(frame: *mut InterruptFrame) -> *mut InterruptFrame {
    let (vector, error_code, rip, cs) = unsafe {
        let frame_ref = &*frame;
        (frame_ref.vector, frame_ref.error_code, frame_ref.rip, frame_ref.cs)
    };
    serial::emergency_println(format_args!(
        "[EXCP] vector={} error={:#x} rip={:#018x} cs={:#x}",
        vector, error_code, rip, cs
    ));

    if vector == 3 || vector == SPURIOUS_VECTOR as u64 {
        return frame;
    }

    if vector == 14 {
        let cr2: u64;
        unsafe { asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack)) };
        serial::emergency_println(format_args!("[EXCP] page-fault address={cr2:#018x}"));
    }

    if cs & 3 == 3 && process::runtime_active() {
        return process::fault_current(
            frame.cast::<crate::user::UserTrapFrame>(),
            vector,
            error_code,
        )
        .cast::<InterruptFrame>();
    }

    serial::emergency_println(format_args!("[HALT] Fatal K11 kernel exception"));
    halt_forever();
}

#[unsafe(no_mangle)]
pub extern "C" fn weave_device_interrupt_dispatch(frame:*mut InterruptFrame)->*mut InterruptFrame {
    let vector=unsafe{(*frame).vector as u8};
    if crate::forgebus::dispatch_interrupt(vector).is_err(){crate::kernel_runtime::with_runtime(|r|r.interrupts.record_spurious(vector));}
    crate::arch::x86_64::apic::end_of_interrupt();
    frame
}

global_asm!(include_str!("interrupts.S"));
global_asm!(include_str!("device_interrupts.S"));
