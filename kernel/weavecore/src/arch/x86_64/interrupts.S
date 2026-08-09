.intel_syntax noprefix
.section .text
.extern weave_exception_dispatch
.extern weave_schedule_dispatch
.extern weave_syscall_dispatch

.global weave_isr_nmi
weave_isr_nmi:
    push 0
    push 2
    jmp weave_exception_common

.global weave_isr_breakpoint
weave_isr_breakpoint:
    push 0
    push 3
    jmp weave_exception_common

.global weave_isr_invalid_opcode
weave_isr_invalid_opcode:
    push 0
    push 6
    jmp weave_exception_common

.global weave_isr_device_not_available
weave_isr_device_not_available:
    push 0
    push 7
    jmp weave_exception_common

.global weave_isr_double_fault
weave_isr_double_fault:
    push 8
    jmp weave_exception_common

# #TS, #NP and #SS already have a hardware-pushed error code. Push only
# the vector so the common frame remains vector,error,rip,cs,rflags.
.global weave_isr_invalid_tss
weave_isr_invalid_tss:
    push 10
    jmp weave_exception_common

.global weave_isr_segment_not_present
weave_isr_segment_not_present:
    push 11
    jmp weave_exception_common

.global weave_isr_stack_segment
weave_isr_stack_segment:
    push 12
    jmp weave_exception_common

.global weave_isr_general_protection
weave_isr_general_protection:
    push 13
    jmp weave_exception_common

.global weave_isr_page_fault
weave_isr_page_fault:
    push 14
    jmp weave_exception_common

.global weave_isr_machine_check
weave_isr_machine_check:
    push 0
    push 18
    jmp weave_exception_common

# #CP carries a hardware error code. This catches CET/IBT/shadow-stack
# failures explicitly instead of allowing a missing gate to obscure them.
.global weave_isr_control_protection
weave_isr_control_protection:
    push 21
    jmp weave_exception_common

.global weave_isr_spurious
weave_isr_spurious:
    push 0
    push 255
    jmp weave_exception_common

weave_exception_common:
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    mov rdi, rsp
    and rsp, -16
    call weave_exception_dispatch
    mov rsp, rax

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax
    add rsp, 16
    iretq

.global weave_isr_timer
weave_isr_timer:
    push 0
    push 64
    jmp weave_schedule_common

.global weave_isr_reschedule
weave_isr_reschedule:
    push 0
    push 65
    jmp weave_schedule_common

weave_schedule_common:
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    mov rdi, rsp
    and rsp, -16
    call weave_schedule_dispatch
    mov rsp, rax

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax
    add rsp, 16
    iretq

.global weave_isr_syscall
weave_isr_syscall:
    push 0
    push 128
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    mov rdi, rsp
    and rsp, -16
    call weave_syscall_dispatch
    mov rsp, rax

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax
    add rsp, 16
    iretq
