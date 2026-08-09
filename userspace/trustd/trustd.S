.intel_syntax noprefix
.include "userspace/include/twabi.inc"
.section .text
.global _start
_start:
    TW_WRITE TW_CONSOLE_HANDLE, banner, BANNER_LEN
loop:
    mov rax, TW_SYS_YIELD
    int 0x80
    jmp loop
.section .rodata
banner:
    .ascii "[trust] Titan Trust Service online"
banner_end:
.equ BANNER_LEN, banner_end - banner
