.intel_syntax noprefix
.include "userspace/include/twabi.inc"
.section .text
.global _start
_start:
    TW_WRITE TW_CONSOLE_HANDLE, banner, BANNER_LEN
    mov rax, TW_SYS_SYSTEM_QUERY
    mov rdi, 3
    int 0x80
    TW_EXIT 0
    ud2

.section .rodata
banner:
    .ascii "[init] disk-backed Titanweave service manager online"
banner_end:
.equ BANNER_LEN, banner_end - banner
