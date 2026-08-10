.intel_syntax noprefix
.include "userspace/include/twabi.inc"

.equ AUDIO_ABI_INFO_SIZE, 48
.equ AUDIO_DEVICE_INFO_SIZE, 72
.equ AUDIO_ENDPOINT_INFO_SIZE, 80
.equ AUDIO_REQUEST_SIZE, 56
.equ AUDIO_RESPONSE_SIZE, 64
.equ ERROR_WOULD_BLOCK, -4
.equ ERROR_NOT_FOUND, -7
.equ ERROR_INVALID_STATE, -11
.equ AUDIO_BACKEND_HDA, 1
.equ AUDIO_DIRECTION_PLAYBACK, 1
.equ AUDIO_DIRECTION_CAPTURE, 2
.equ AUDIO_FORMAT_S16, 1
.equ AUDIO_BUFFER_RW, 3
.equ PCM_RATE, 48000
.equ PCM_CHANNELS, 2
.equ PCM_PERIOD_FRAMES, 256
.equ PCM_BUFFER_FRAMES, 1024
.equ PCM_FRAME_STRIDE, 4
.equ PCM_BUFFER_BYTES, 4096
.equ PCM_GEOMETRY, 0x0000040000000100

.section .text
.global _start
_start:
    TW_WRITE TW_CONSOLE_HANDLE, banner, BANNER_LEN

    # Confirm the frozen K15.2 ABI before touching any audio object.
    mov rax, TW_SYS_AUDIO_ABI_QUERY
    lea rdi, [rip + abi_info]
    mov rsi, AUDIO_ABI_INFO_SIZE
    int 0x80
    cmp rax, AUDIO_ABI_INFO_SIZE
    jne fatal
    cmp dword ptr [rip + abi_info], TW_FORGEAUDIO_ABI_VERSION
    jne fatal

    # Enumerate the first real audio device. Older K15.1-K15.3 runners have
    # none by design, so ForgeAudioD remains a dormant persistent service there.
    mov rax, TW_SYS_AUDIO_ENUMERATE
    mov rdi, TW_AUDIO_OBJECT_DEVICE
    xor rsi, rsi
    xor rdx, rdx
    lea r10, [rip + device_info]
    mov r8, AUDIO_DEVICE_INFO_SIZE
    int 0x80
    cmp rax, ERROR_NOT_FOUND
    je dormant
    cmp rax, AUDIO_DEVICE_INFO_SIZE
    jne fatal
    cmp byte ptr [rip + device_info + 27], AUDIO_BACKEND_HDA
    cmp dword ptr [rip + device_info + 28], 2
    jb fatal
    jne fatal

    # Resolve both endpoints and classify them by direction rather than index.
    mov r12, qword ptr [rip + device_info]
    xor r13d, r13d
endpoint_scan:
    cmp r13d, 2
    jae endpoint_scan_done
    mov rax, TW_SYS_AUDIO_ENUMERATE
    mov rdi, TW_AUDIO_OBJECT_ENDPOINT
    mov rsi, r12
    mov rdx, r13
    lea r10, [rip + endpoint_info]
    mov r8, AUDIO_ENDPOINT_INFO_SIZE
    int 0x80
    cmp rax, AUDIO_ENDPOINT_INFO_SIZE
    jne fatal
    mov eax, dword ptr [rip + endpoint_info + 20]
    cmp eax, AUDIO_DIRECTION_PLAYBACK
    je endpoint_playback
    cmp eax, AUDIO_DIRECTION_CAPTURE
    je endpoint_capture
    jmp endpoint_next
endpoint_playback:
    mov rax, qword ptr [rip + endpoint_info]
    mov qword ptr [rip + playback_endpoint_id], rax
    jmp endpoint_next
endpoint_capture:
    mov rax, qword ptr [rip + endpoint_info]
    mov qword ptr [rip + capture_endpoint_id], rax
endpoint_next:
    inc r13d
    jmp endpoint_scan
endpoint_scan_done:
    cmp qword ptr [rip + playback_endpoint_id], 0
    je fatal
    cmp qword ptr [rip + capture_endpoint_id], 0
    je fatal

    # Register exactly one privileged userspace ForgeAudioD instance.
    mov rax, TW_SYS_AUDIO_SERVER_CONTROL
    mov rdi, TW_AUDIO_SERVER_OP_REGISTER
    xor rsi, rsi
    xor rdx, rdx
    xor r10, r10
    xor r8, r8
    int 0x80
    test rax, rax
    js fatal
    mov qword ptr [rip + server_pid], rax

    # Own the physical HDA device object.
    call clear_request
    mov dword ptr [rip + request + 4], TW_AUDIO_OP_OPEN_DEVICE
    mov rax, qword ptr [rip + device_info]
    mov qword ptr [rip + request + 16], rax
    call audio_control
    call require_response
    mov eax, dword ptr [rip + response + 8]
    mov dword ptr [rip + device_handle], eax

    # Own and prepare the playback stream at the K15.5 baseline PCM format.
    mov rax, qword ptr [rip + playback_endpoint_id]
    mov edi, AUDIO_DIRECTION_PLAYBACK
    call open_configure_prepare_stream
    mov dword ptr [rip + playback_stream_handle], eax

    # Own and prepare the capture stream.
    mov rax, qword ptr [rip + capture_endpoint_id]
    mov edi, AUDIO_DIRECTION_CAPTURE
    call open_configure_prepare_stream
    mov dword ptr [rip + capture_stream_handle], eax

    # Own one bounded server buffer for each direction.
    call create_server_buffer
    mov dword ptr [rip + playback_buffer_handle], eax
    call create_server_buffer
    mov dword ptr [rip + capture_buffer_handle], eax

    # Own the control clock, telemetry event queue, and control fence.
    call clear_request
    mov dword ptr [rip + request + 4], TW_AUDIO_OP_CREATE_CLOCK
    mov qword ptr [rip + request + 24], PCM_RATE
    mov qword ptr [rip + request + 32], 1
    call audio_control
    call require_response
    mov eax, dword ptr [rip + response + 8]
    mov dword ptr [rip + clock_handle], eax

    call clear_request
    mov dword ptr [rip + request + 4], TW_AUDIO_OP_CREATE_EVENT
    call audio_control
    call require_response
    mov eax, dword ptr [rip + response + 8]
    mov dword ptr [rip + event_handle], eax

    call clear_request
    mov dword ptr [rip + request + 4], TW_AUDIO_OP_CREATE_FENCE
    mov qword ptr [rip + request + 24], 1
    call audio_control
    call require_response
    mov eax, dword ptr [rip + response + 8]
    mov dword ptr [rip + fence_handle], eax

    # Telemetry proof: read the owned clock, observe an honestly empty event
    # queue, and inspect the unsignaled fence target through the frozen ABI.
    call clear_request
    mov dword ptr [rip + request + 4], TW_AUDIO_OP_QUERY_POSITION
    mov eax, dword ptr [rip + clock_handle]
    mov dword ptr [rip + request + 8], eax
    call audio_control
    call require_response
    mov rax, qword ptr [rip + response + 32]
    mov qword ptr [rip + telemetry_clock_tick], rax

    call clear_request
    mov dword ptr [rip + request + 4], TW_AUDIO_OP_POLL_EVENT
    mov eax, dword ptr [rip + event_handle]
    mov dword ptr [rip + request + 8], eax
    call audio_control
    cmp rax, ERROR_WOULD_BLOCK
    jne fatal

    call clear_request
    mov dword ptr [rip + request + 4], TW_AUDIO_OP_QUERY_FENCE
    mov eax, dword ptr [rip + fence_handle]
    mov dword ptr [rip + request + 8], eax
    call audio_control
    call require_response
    cmp qword ptr [rip + response + 32], 1
    jne fatal
    cmp qword ptr [rip + response + 40], 0
    jne fatal

    # K15.6 recovery proof: an illegal start must be rejected, then the daemon
    # closes that bad control object, rebuilds it, configures/prepares it, and
    # releases it without disturbing the two production-owned streams.
    mov rax, qword ptr [rip + playback_endpoint_id]
    call open_stream_only
    mov dword ptr [rip + temp_stream_handle], eax
    call clear_request
    mov dword ptr [rip + request + 4], TW_AUDIO_OP_START_STREAM
    mov eax, dword ptr [rip + temp_stream_handle]
    mov dword ptr [rip + request + 8], eax
    call audio_control
    cmp rax, ERROR_INVALID_STATE
    jne fatal
    mov eax, dword ptr [rip + temp_stream_handle]
    call close_handle

    mov rax, qword ptr [rip + playback_endpoint_id]
    mov edi, AUDIO_DIRECTION_PLAYBACK
    call open_configure_prepare_stream
    mov dword ptr [rip + temp_stream_handle], eax
    mov eax, dword ptr [rip + temp_stream_handle]
    call close_handle
    mov dword ptr [rip + recovery_count], 1

    # Materialize the bounded two-route userspace control plane. K15.8 will
    # implement DSP graph nodes; K15.6 owns only route/control metadata.
    mov rax, qword ptr [rip + playback_endpoint_id]
    mov qword ptr [rip + route_table + 0], rax
    mov eax, dword ptr [rip + playback_stream_handle]
    mov dword ptr [rip + route_table + 8], eax
    mov eax, dword ptr [rip + playback_buffer_handle]
    mov dword ptr [rip + route_table + 12], eax
    mov rax, qword ptr [rip + capture_endpoint_id]
    mov qword ptr [rip + route_table + 16], rax
    mov eax, dword ptr [rip + capture_stream_handle]
    mov dword ptr [rip + route_table + 24], eax
    mov eax, dword ptr [rip + capture_buffer_handle]
    mov dword ptr [rip + route_table + 28], eax
    mov dword ptr [rip + route_count], 2
    mov dword ptr [rip + graph_generation], 1

    TW_WRITE TW_CONSOLE_HANDLE, ownership_msg, OWNERSHIP_MSG_LEN
    TW_WRITE TW_CONSOLE_HANDLE, objects_msg, OBJECTS_MSG_LEN
    TW_WRITE TW_CONSOLE_HANDLE, routes_msg, ROUTES_MSG_LEN
    TW_WRITE TW_CONSOLE_HANDLE, telemetry_msg, TELEMETRY_MSG_LEN
    TW_WRITE TW_CONSOLE_HANDLE, recovery_msg, RECOVERY_MSG_LEN

    # Publish server ownership. Kernel cross-validates the live process handle
    # table and ForgeAudio object ownership rather than trusting these counters.
    mov rax, TW_SYS_AUDIO_SERVER_CONTROL
    mov rdi, TW_AUDIO_SERVER_OP_PUBLISH
    mov esi, dword ptr [rip + device_handle]
    mov edx, dword ptr [rip + route_count]
    mov r10d, dword ptr [rip + graph_generation]
    mov r8d, dword ptr [rip + recovery_count]
    int 0x80
    test rax, rax
    js fatal
    mov qword ptr [rip + ownership_packed], rax
    TW_WRITE TW_CONSOLE_HANDLE, ready_msg, READY_MSG_LEN

    # A real persistent daemon must survive a scheduler handoff and execute
    # again before K6 is allowed to finish userspace qualification.
    mov rax, TW_SYS_YIELD
    int 0x80
    mov rax, TW_SYS_AUDIO_SERVER_CONTROL
    mov rdi, TW_AUDIO_SERVER_OP_HEARTBEAT
    mov rsi, 1
    xor rdx, rdx
    xor r10, r10
    xor r8, r8
    int 0x80
    cmp rax, 1
    jne fatal
    TW_WRITE TW_CONSOLE_HANDLE, heartbeat_msg, HEARTBEAT_MSG_LEN

service_loop:
    mov rax, TW_SYS_YIELD
    int 0x80
    jmp service_loop

# Open one stream under the already-owned device. Input: RAX endpoint id.
# Output: EAX stream handle.
open_stream_only:
    push rbx
    mov rbx, rax
    call clear_request
    mov dword ptr [rip + request + 4], TW_AUDIO_OP_OPEN_STREAM
    mov eax, dword ptr [rip + device_handle]
    mov dword ptr [rip + request + 8], eax
    mov qword ptr [rip + request + 16], rbx
    call audio_control
    call require_response
    mov eax, dword ptr [rip + response + 8]
    pop rbx
    ret

# Input: RAX endpoint id, EDI direction. Output: EAX prepared stream handle.
open_configure_prepare_stream:
    push rbx
    push r12
    mov rbx, rax
    mov r12d, edi
    mov rax, rbx
    call open_stream_only
    mov ebx, eax

    call clear_request
    mov dword ptr [rip + request + 4], TW_AUDIO_OP_CONFIGURE_STREAM
    mov dword ptr [rip + request + 8], ebx
    mov eax, r12d
    mov qword ptr [rip + request + 16], rax
    mov qword ptr [rip + request + 24], AUDIO_FORMAT_S16
    mov qword ptr [rip + request + 32], PCM_RATE
    mov qword ptr [rip + request + 40], PCM_CHANNELS
    mov rax, PCM_GEOMETRY
    mov qword ptr [rip + request + 48], rax
    call audio_control
    call require_response
    cmp dword ptr [rip + response + 12], 2
    jne fatal

    call clear_request
    mov dword ptr [rip + request + 4], TW_AUDIO_OP_PREPARE_STREAM
    mov dword ptr [rip + request + 8], ebx
    call audio_control
    call require_response
    cmp dword ptr [rip + response + 12], 3
    jne fatal
    mov eax, ebx
    pop r12
    pop rbx
    ret

create_server_buffer:
    call clear_request
    mov dword ptr [rip + request + 4], TW_AUDIO_OP_CREATE_BUFFER
    mov dword ptr [rip + request + 12], AUDIO_BUFFER_RW
    mov qword ptr [rip + request + 24], PCM_BUFFER_BYTES
    mov qword ptr [rip + request + 32], PCM_FRAME_STRIDE
    call audio_control
    call require_response
    cmp qword ptr [rip + response + 32], PCM_BUFFER_BYTES
    jne fatal
    cmp qword ptr [rip + response + 40], PCM_FRAME_STRIDE
    jne fatal
    mov eax, dword ptr [rip + response + 8]
    ret

# Input EAX = closeable audio handle.
close_handle:
    push rbx
    mov ebx, eax
    call clear_request
    mov dword ptr [rip + request + 4], TW_AUDIO_OP_CLOSE_OBJECT
    mov dword ptr [rip + request + 8], ebx
    call audio_control
    call require_response
    pop rbx
    ret

audio_control:
    mov rax, TW_SYS_AUDIO_CONTROL
    lea rdi, [rip + request]
    mov rsi, AUDIO_REQUEST_SIZE
    lea rdx, [rip + response]
    mov r10, AUDIO_RESPONSE_SIZE
    xor r8, r8
    int 0x80
    ret

require_response:
    cmp rax, AUDIO_RESPONSE_SIZE
    jne fatal
    cmp dword ptr [rip + response], TW_FORGEAUDIO_ABI_VERSION
    jne fatal
    ret

clear_request:
    lea r11, [rip + request]
    xor rax, rax
    mov qword ptr [r11 + 0], rax
    mov qword ptr [r11 + 8], rax
    mov qword ptr [r11 + 16], rax
    mov qword ptr [r11 + 24], rax
    mov qword ptr [r11 + 32], rax
    mov qword ptr [r11 + 40], rax
    mov qword ptr [r11 + 48], rax
    mov dword ptr [r11 + 0], TW_FORGEAUDIO_ABI_VERSION
    lea r11, [rip + response]
    mov qword ptr [r11 + 0], rax
    mov qword ptr [r11 + 8], rax
    mov qword ptr [r11 + 16], rax
    mov qword ptr [r11 + 24], rax
    mov qword ptr [r11 + 32], rax
    mov qword ptr [r11 + 40], rax
    mov qword ptr [r11 + 48], rax
    mov qword ptr [r11 + 56], rax
    ret

dormant:
    TW_WRITE TW_CONSOLE_HANDLE, dormant_msg, DORMANT_MSG_LEN
dormant_loop:
    mov rax, TW_SYS_YIELD
    int 0x80
    jmp dormant_loop

fatal:
    TW_WRITE TW_CONSOLE_HANDLE, fail_msg, FAIL_MSG_LEN
    TW_EXIT 1
    ud2

.section .rodata
banner:
    .ascii "[USER] [forgeaudiod] K15.6 ForgeAudioD userspace audio server starting"
banner_end:
.equ BANNER_LEN, banner_end - banner
ownership_msg:
    .ascii "[USER] [forgeaudiod] K15.6 device ownership: HDA=true playback=true capture=true singleton=true"
ownership_msg_end:
.equ OWNERSHIP_MSG_LEN, ownership_msg_end - ownership_msg
objects_msg:
    .ascii "[USER] [forgeaudiod] K15.6 objects: streams=2 buffers=2 clocks=1 events=1 fences=1 prepared=true"
objects_msg_end:
.equ OBJECTS_MSG_LEN, objects_msg_end - objects_msg
routes_msg:
    .ascii "[USER] [forgeaudiod] K15.6 control plane: routes=2 graph_generation=1 bounded=true no_mixing=true no_resampling=true"
routes_msg_end:
.equ ROUTES_MSG_LEN, routes_msg_end - routes_msg
telemetry_msg:
    .ascii "[USER] [forgeaudiod] K15.6 telemetry: clock=true event_queue=true empty_event=true fence=true target=1 completed=0"
telemetry_msg_end:
.equ TELEMETRY_MSG_LEN, telemetry_msg_end - telemetry_msg
recovery_msg:
    .ascii "[USER] [forgeaudiod] K15.6 recovery: invalid_start_rejected=true stream_rebuilt=true recoveries=1"
recovery_msg_end:
.equ RECOVERY_MSG_LEN, recovery_msg_end - recovery_msg
ready_msg:
    .ascii "[USER] [forgeaudiod] K15.6 ForgeAudioD ready: device=true streams=true routing=true clocks=true buffers=true telemetry=true recovery=true"
ready_msg_end:
.equ READY_MSG_LEN, ready_msg_end - ready_msg
heartbeat_msg:
    .ascii "[USER] [forgeaudiod] K15.6 persistent heartbeat: sequence=1 scheduler_return=true"
heartbeat_msg_end:
.equ HEARTBEAT_MSG_LEN, heartbeat_msg_end - heartbeat_msg
dormant_msg:
    .ascii "[USER] [forgeaudiod] no audio hardware registered; dormant service retained for pre-HDA qualification"
dormant_msg_end:
.equ DORMANT_MSG_LEN, dormant_msg_end - dormant_msg
fail_msg:
    .ascii "[FAIL] ForgeAudioD K15.6 userspace initialization failed"
fail_msg_end:
.equ FAIL_MSG_LEN, fail_msg_end - fail_msg

.section .bss
.align 16
abi_info: .zero AUDIO_ABI_INFO_SIZE
device_info: .zero AUDIO_DEVICE_INFO_SIZE
endpoint_info: .zero AUDIO_ENDPOINT_INFO_SIZE
request: .zero AUDIO_REQUEST_SIZE
response: .zero AUDIO_RESPONSE_SIZE
playback_endpoint_id: .quad 0
capture_endpoint_id: .quad 0
server_pid: .quad 0
ownership_packed: .quad 0
telemetry_clock_tick: .quad 0
device_handle: .long 0
playback_stream_handle: .long 0
capture_stream_handle: .long 0
playback_buffer_handle: .long 0
capture_buffer_handle: .long 0
clock_handle: .long 0
event_handle: .long 0
fence_handle: .long 0
temp_stream_handle: .long 0
recovery_count: .long 0
route_count: .long 0
graph_generation: .long 0
route_table: .zero 32
