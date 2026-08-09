use crate::arch::x86_64::gdt::{USER_CODE_SELECTOR, USER_DATA_SELECTOR};
use core::arch::global_asm;
use core::mem::size_of;

#[repr(C)]
pub struct UserTrapFrame {
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
    pub user_rsp: u64,
    pub user_ss: u64,
}

const _: [(); 176] = [(); size_of::<UserTrapFrame>()];

impl UserTrapFrame {
    pub const fn initial(entry: u64, stack_pointer: u64) -> Self {
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
            vector: 0,
            error_code: 0,
            rip: entry,
            cs: USER_CODE_SELECTOR as u64,
            rflags: 0x202,
            user_rsp: stack_pointer,
            user_ss: USER_DATA_SELECTOR as u64,
        }
    }
}

unsafe extern "C" {
    fn weave_enter_user_frame(frame: *const UserTrapFrame, cr3: u64) -> !;
    fn weave_return_to_kernel(kernel_cr3: u64, kernel_stack_top: u64, result: u64) -> !;
}

pub unsafe fn enter_frame(frame: *const UserTrapFrame, cr3: u64) -> ! {
    unsafe { weave_enter_user_frame(frame, cr3) }
}

pub unsafe fn return_to_kernel(kernel_cr3: u64, kernel_stack_top: u64, result: u64) -> ! {
    unsafe { weave_return_to_kernel(kernel_cr3, kernel_stack_top, result) }
}

global_asm!(include_str!("user_mode.S"));
