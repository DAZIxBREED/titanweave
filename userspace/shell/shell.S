.intel_syntax noprefix
.include "userspace/include/twabi.inc"
.section .text
.global _start
_start:
    TW_WRITE TW_CONSOLE_HANDLE, banner, BANNER_LEN

    TW_WRITE TW_CONSOLE_HANDLE, cmd_dir, CMD_DIR_LEN
    mov rax, TW_SYS_SYSTEM_QUERY
    mov rdi, 0
    int 0x80

    TW_WRITE TW_CONSOLE_HANDLE, cmd_ps, CMD_PS_LEN
    mov rax, TW_SYS_SYSTEM_QUERY
    mov rdi, 1
    int 0x80

    TW_WRITE TW_CONSOLE_HANDLE, cmd_services, CMD_SERVICES_LEN
    mov rax, TW_SYS_SYSTEM_QUERY
    mov rdi, 2
    int 0x80

    TW_WRITE TW_CONSOLE_HANDLE, cmd_status, CMD_STATUS_LEN
    mov rax, TW_SYS_SYSTEM_QUERY
    mov rdi, 3
    int 0x80

    TW_WRITE TW_CONSOLE_HANDLE, cmd_uptime, CMD_UPTIME_LEN
    mov rax, TW_SYS_SYSTEM_QUERY
    mov rdi, 4
    int 0x80

    TW_WRITE TW_CONSOLE_HANDLE, cmd_display, CMD_DISPLAY_LEN
    mov rax, TW_SYS_DISPLAY_QUERY
    xor rdi, rdi
    int 0x80

    TW_WRITE TW_CONSOLE_HANDLE, done, DONE_LEN
    TW_EXIT 0
    ud2

.section .rodata
banner:
    .ascii "[shell] Titanweave command console ready C:\\>"
banner_end:
.equ BANNER_LEN, banner_end - banner
cmd_dir:
    .ascii "C:\\> dir C:\\SYSTEM\\SERVICES"
cmd_dir_end:
.equ CMD_DIR_LEN, cmd_dir_end - cmd_dir
cmd_ps:
    .ascii "C:\\> ps"
cmd_ps_end:
.equ CMD_PS_LEN, cmd_ps_end - cmd_ps
cmd_services:
    .ascii "C:\\> services"
cmd_services_end:
.equ CMD_SERVICES_LEN, cmd_services_end - cmd_services
cmd_status:
    .ascii "C:\\> type \\Shared\\boot-status"
cmd_status_end:
.equ CMD_STATUS_LEN, cmd_status_end - cmd_status
cmd_uptime:
    .ascii "C:\\> uptime"
cmd_uptime_end:
.equ CMD_UPTIME_LEN, cmd_uptime_end - cmd_uptime
cmd_display:
    .ascii "C:\\> display"
cmd_display_end:
.equ CMD_DISPLAY_LEN, cmd_display_end - cmd_display
done:
    .ascii "[shell] scripted K13 console session complete"
done_end:
.equ DONE_LEN, done_end - done
