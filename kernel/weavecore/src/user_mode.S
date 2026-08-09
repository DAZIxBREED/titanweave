.intel_syntax noprefix
.section .text

.global weave_enter_user_frame
weave_enter_user_frame:
    cli
    mov cr3, rsi
    mov ax, 0x1b
    mov ds, ax
    mov es, ax
    mov rsp, rdi
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

.global weave_return_to_kernel
.extern weave_user_exit_resume
weave_return_to_kernel:
    cli
    mov cr3, rdi
    mov rsp, rsi
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    and rsp, -16
    sub rsp, 8
    mov qword ptr [rsp], 0
    mov rdi, rdx
    jmp weave_user_exit_resume
