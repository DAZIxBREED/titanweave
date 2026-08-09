.intel_syntax noprefix
.section .ap_trampoline, "ax"
.balign 4096
.global weave_ap_trampoline_start
weave_ap_trampoline_start:
.equ TW_AP_BASE,         0x8000
.equ TW_AP_GDTR,         0x8728
.equ TW_AP_PARAM_CR3,    0x8800
.equ TW_AP_PARAM_STACK,  0x8808
.equ TW_AP_PARAM_BOOT,   0x8810
.equ TW_AP_PARAM_ENTRY,  0x8818
.equ TW_AP_PARAM_CPU,    0x8820
.code16
    cli
    cld
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7c00
    lgdt [TW_AP_GDTR]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    .byte 0xea
    .word 0x8080
    .word 0x0008
.org 0x80
.code32
weave_ap_protected_mode:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax
    mov eax, dword ptr [TW_AP_PARAM_CR3]
    mov cr3, eax
    mov ecx, 0xc0000080
    rdmsr
    or eax, 1 << 8
    wrmsr
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax
    .byte 0xea
    .long 0x8100
    .word 0x0018
.org 0x100
.code64
weave_ap_long_mode:
    mov ax, 0x20
    mov ds, ax
    mov es, ax
    mov ss, ax
    xor ebp, ebp
    mov rsp, qword ptr [TW_AP_PARAM_STACK]
    mov rdi, qword ptr [TW_AP_PARAM_BOOT]
    mov rsi, qword ptr [TW_AP_PARAM_CPU]
    mov rax, qword ptr [TW_AP_PARAM_ENTRY]
    jmp rax
.org 0x700
weave_ap_trampoline_gdt:
    .quad 0x0000000000000000
    .quad 0x00cf9a000000ffff
    .quad 0x00cf92000000ffff
    .quad 0x00af9a000000ffff
    .quad 0x00cf92000000ffff
weave_ap_trampoline_gdtr:
    .word 39
    .long 0x8700
.org 0x800
    .quad 0
    .quad 0
    .quad 0
    .quad 0
    .quad 0
.org 0x1000
.global weave_ap_trampoline_end
weave_ap_trampoline_end:
.code64
