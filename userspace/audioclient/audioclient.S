.intel_syntax noprefix
.include "userspace/include/twabi.inc"

.equ ERROR_WOULD_BLOCK, -4
.equ ERROR_NOT_READY, -8
.equ TRANSPORT_SESSION_EXPECTED, 1
.equ TRANSPORT_GENERATION_EXPECTED, 1
.equ COMMAND_OPCODE_CLIENT, 1
.equ COMMAND_OPCODE_ACK, 2
.equ AUDIO_BLOCKS_TOTAL, 12

.section .text
.global _start
_start:
    TW_WRITE TW_CONSOLE_HANDLE, banner, BANNER_LEN

    # Attach only after the persistent ForgeAudioD K15.6 ownership/heartbeat
    # proof exists. Pre-HDA qualification has no server and remains dormant.
attach_retry:
    mov rax, TW_SYS_AUDIO_TRANSPORT_CONTROL
    mov rdi, TW_AUDIO_TRANSPORT_CLIENT_ATTACH
    xor rsi, rsi
    xor rdx, rdx
    xor r10, r10
    xor r8, r8
    int 0x80
    cmp rax, ERROR_NOT_READY
    jne attach_result
    call yield_once
    jmp attach_retry
attach_result:
    test rax, rax
    js fatal
    mov dword ptr [rip + session_id], eax
    shr rax, 32
    mov dword ptr [rip + generation], eax
    cmp dword ptr [rip + session_id], TRANSPORT_SESSION_EXPECTED
    jne fatal
    cmp dword ptr [rip + generation], TRANSPORT_GENERATION_EXPECTED
    jne fatal
    TW_WRITE TW_CONSOLE_HANDLE, attached_msg, ATTACHED_MSG_LEN

    # Fill the complete client->server command queue without yielding.
    mov r12d, 1
command_push_loop:
    cmp r12d, TW_AUDIO_TRANSPORT_COMMAND_DEPTH + 1
    jae command_full_probe
    call prepare_client_command
    mov rax, TW_SYS_AUDIO_TRANSPORT_CONTROL
    mov rdi, TW_AUDIO_TRANSPORT_CLIENT_PUSH_COMMAND
    mov esi, dword ptr [rip + session_id]
    mov edx, dword ptr [rip + generation]
    lea r10, [rip + command]
    mov r8, TW_AUDIO_TRANSPORT_COMMAND_BYTES
    int 0x80
    cmp rax, r12
    jne fatal
    inc r12d
    jmp command_push_loop

command_full_probe:
    # Entry 17 must fail while the bounded depth-16 queue is full.
    call prepare_client_command
    mov rax, TW_SYS_AUDIO_TRANSPORT_CONTROL
    mov rdi, TW_AUDIO_TRANSPORT_CLIENT_PUSH_COMMAND
    mov esi, dword ptr [rip + session_id]
    mov edx, dword ptr [rip + generation]
    lea r10, [rip + command]
    mov r8, TW_AUDIO_TRANSPORT_COMMAND_BYTES
    int 0x80
    cmp rax, ERROR_WOULD_BLOCK
    jne fatal
    call yield_once

    # Read all sixteen server acknowledgements in exact sequence order.
    mov r12d, 1
command_ack_loop:
    cmp r12d, TW_AUDIO_TRANSPORT_COMMAND_DEPTH + 1
    jae command_empty_probe
command_ack_retry:
    mov rax, TW_SYS_AUDIO_TRANSPORT_CONTROL
    mov rdi, TW_AUDIO_TRANSPORT_CLIENT_POP_COMMAND
    mov esi, dword ptr [rip + session_id]
    mov edx, dword ptr [rip + generation]
    lea r10, [rip + command]
    mov r8, TW_AUDIO_TRANSPORT_COMMAND_BYTES
    int 0x80
    cmp rax, ERROR_WOULD_BLOCK
    jne command_ack_have
    call yield_once
    jmp command_ack_retry
command_ack_have:
    cmp rax, r12
    jne fatal
    cmp qword ptr [rip + command + 0], r12
    jne fatal
    mov eax, dword ptr [rip + generation]
    cmp dword ptr [rip + command + 8], eax
    jne fatal
    cmp dword ptr [rip + command + 12], COMMAND_OPCODE_ACK
    jne fatal
    inc r12d
    jmp command_ack_loop

command_empty_probe:
    mov rax, TW_SYS_AUDIO_TRANSPORT_CONTROL
    mov rdi, TW_AUDIO_TRANSPORT_CLIENT_POP_COMMAND
    mov esi, dword ptr [rip + session_id]
    mov edx, dword ptr [rip + generation]
    lea r10, [rip + command]
    mov r8, TW_AUDIO_TRANSPORT_COMMAND_BYTES
    int 0x80
    cmp rax, ERROR_WOULD_BLOCK
    jne fatal
    TW_WRITE TW_CONSOLE_HANDLE, command_msg, COMMAND_MSG_LEN

    # Three complete four-slot laps prove wraparound while preserving SPSC
    # ownership. Each producer lap also proves bounded-full semantics.
    xor r13d, r13d                 # batch 0..2
audio_batch_loop:
    cmp r13d, 3
    jae audio_done
    xor r14d, r14d                 # slot 0..3
audio_push_loop:
    cmp r14d, TW_AUDIO_TRANSPORT_RING_SLOTS
    jae audio_full_probe
    mov eax, r13d
    imul eax, TW_AUDIO_TRANSPORT_RING_SLOTS
    add eax, r14d
    inc eax                        # deterministic block id 1..12
    mov dword ptr [rip + current_block_id], eax
    call fill_audio_block
    mov rax, TW_SYS_AUDIO_TRANSPORT_CONTROL
    mov rdi, TW_AUDIO_TRANSPORT_CLIENT_PUSH_PLAYBACK
    mov esi, dword ptr [rip + session_id]
    mov edx, dword ptr [rip + generation]
    lea r10, [rip + audio_block]
    mov r8, TW_AUDIO_TRANSPORT_BLOCK_BYTES
    int 0x80
    mov ecx, dword ptr [rip + current_block_id]
    cmp rax, rcx
    jne fatal
    inc r14d
    jmp audio_push_loop

audio_full_probe:
    mov rax, TW_SYS_AUDIO_TRANSPORT_CONTROL
    mov rdi, TW_AUDIO_TRANSPORT_CLIENT_PUSH_PLAYBACK
    mov esi, dword ptr [rip + session_id]
    mov edx, dword ptr [rip + generation]
    lea r10, [rip + audio_block]
    mov r8, TW_AUDIO_TRANSPORT_BLOCK_BYTES
    int 0x80
    cmp rax, ERROR_WOULD_BLOCK
    jne fatal
    call yield_once

    xor r14d, r14d
audio_capture_loop:
    cmp r14d, TW_AUDIO_TRANSPORT_RING_SLOTS
    jae audio_empty_probe
    mov eax, r13d
    imul eax, TW_AUDIO_TRANSPORT_RING_SLOTS
    add eax, r14d
    inc eax
    mov dword ptr [rip + current_block_id], eax
audio_capture_retry:
    mov rax, TW_SYS_AUDIO_TRANSPORT_CONTROL
    mov rdi, TW_AUDIO_TRANSPORT_CLIENT_POP_CAPTURE
    mov esi, dword ptr [rip + session_id]
    mov edx, dword ptr [rip + generation]
    lea r10, [rip + capture_block]
    mov r8, TW_AUDIO_TRANSPORT_BLOCK_BYTES
    int 0x80
    cmp rax, ERROR_WOULD_BLOCK
    jne audio_capture_have
    call yield_once
    jmp audio_capture_retry
audio_capture_have:
    mov ecx, dword ptr [rip + current_block_id]
    cmp rax, rcx
    jne fatal
    call verify_capture_block
    inc r14d
    jmp audio_capture_loop

audio_empty_probe:
    mov rax, TW_SYS_AUDIO_TRANSPORT_CONTROL
    mov rdi, TW_AUDIO_TRANSPORT_CLIENT_POP_CAPTURE
    mov esi, dword ptr [rip + session_id]
    mov edx, dword ptr [rip + generation]
    lea r10, [rip + capture_block]
    mov r8, TW_AUDIO_TRANSPORT_BLOCK_BYTES
    int 0x80
    cmp rax, ERROR_WOULD_BLOCK
    jne fatal
    inc r13d
    jmp audio_batch_loop

audio_done:
    TW_WRITE TW_CONSOLE_HANDLE, audio_msg, AUDIO_MSG_LEN
    TW_WRITE TW_CONSOLE_HANDLE, exit_msg, EXIT_MSG_LEN
    # Normal client exit is the dead-client isolation trigger. The kernel must
    # invalidate the generation and wipe only this session; ForgeAudioD lives.
    TW_EXIT 0

prepare_client_command:
    mov qword ptr [rip + command + 0], r12
    mov eax, dword ptr [rip + generation]
    mov dword ptr [rip + command + 8], eax
    mov dword ptr [rip + command + 12], COMMAND_OPCODE_CLIENT
    mov qword ptr [rip + command + 16], r12
    mov qword ptr [rip + command + 24], 0x4b313537
    ret

fill_audio_block:
    mov eax, dword ptr [rip + current_block_id]
    lea rdi, [rip + audio_block]
    mov ecx, TW_AUDIO_TRANSPORT_BLOCK_BYTES
    rep stosb
    ret

verify_capture_block:
    mov eax, dword ptr [rip + current_block_id]
    lea rdi, [rip + capture_block]
    mov ecx, TW_AUDIO_TRANSPORT_BLOCK_BYTES
    repe scasb
    jne fatal
    ret

yield_once:
    mov rax, TW_SYS_YIELD
    int 0x80
    ret

dormant:
    TW_WRITE TW_CONSOLE_HANDLE, dormant_msg, DORMANT_MSG_LEN
dormant_loop:
    call yield_once
    jmp dormant_loop

fatal:
    TW_WRITE TW_CONSOLE_HANDLE, fatal_msg, FATAL_MSG_LEN
    TW_EXIT 1

.section .rodata
banner:
    .ascii "[USER] [audioclient] K15.7 lock-free application transport starting"
.set BANNER_LEN, . - banner
attached_msg:
    .ascii "[USER] [audioclient] K15.7 attached: session=1 generation=1 SPSC=true bounded=true"
.set ATTACHED_MSG_LEN, . - attached_msg
command_msg:
    .ascii "[USER] [audioclient] K15.7 command queue: depth=16 roundtrips=16 full=true empty=true sequence=true"
.set COMMAND_MSG_LEN, . - command_msg
audio_msg:
    .ascii "[USER] [audioclient] K15.7 audio rings: playback_blocks=12 capture_blocks=12 wraps=3 full=true empty=true data_verified=true"
.set AUDIO_MSG_LEN, . - audio_msg
exit_msg:
    .ascii "[USER] [audioclient] K15.7 exiting normally to trigger dead-client isolation"
.set EXIT_MSG_LEN, . - exit_msg
dormant_msg:
    .ascii "[USER] [audioclient] no ForgeAudioD transport available; dormant service retained for pre-HDA qualification"
.set DORMANT_MSG_LEN, . - dormant_msg
fatal_msg:
    .ascii "[USER] [audioclient] K15.7 transport qualification failed"
.set FATAL_MSG_LEN, . - fatal_msg

.section .bss
.align 16
session_id: .skip 4
generation: .skip 4
current_block_id: .skip 4
.align 16
command: .skip TW_AUDIO_TRANSPORT_COMMAND_BYTES
.align 64
audio_block: .skip TW_AUDIO_TRANSPORT_BLOCK_BYTES
.align 64
capture_block: .skip TW_AUDIO_TRANSPORT_BLOCK_BYTES
.section .note.GNU-stack,"",@progbits
