# Titanweave K15.8 ForgeAudio Graph Engine
#
# Userspace-owned, fixed-capacity graph execution proof. K15.8 intentionally
# leaves the qualified K15.1-K15.7 kernel unchanged. Topology compilation is
# performed on ForgeAudioD's control path before any block processing. The
# process path uses fixed storage only: no allocation, locks, sleeps, syscalls
# or topology work occurs inside graph_process_compiled_block.

.equ K15_GRAPH_VERSION, 1
.equ K15_GRAPH_GENERATION, 1
.equ K15_GRAPH_NODE_COUNT, 8
.equ K15_GRAPH_EDGE_COUNT, 8
.equ K15_GRAPH_MAX_NODES, 16
.equ K15_GRAPH_MAX_INPUTS, 4
.equ K15_GRAPH_BLOCKS, 4
.equ K15_GRAPH_BLOCK_BYTES, 1024
.equ K15_GRAPH_SAMPLES, 512
.equ K15_GRAPH_FRAMES, 256
.equ K15_GRAPH_NODE_NONE, 0xff

.equ K15_NODE_INPUT, 1
.equ K15_NODE_OUTPUT, 2
.equ K15_NODE_GAIN, 3
.equ K15_NODE_MIXER, 4
.equ K15_NODE_SPLITTER, 5
.equ K15_NODE_CHANNEL_MAPPER, 6
.equ K15_NODE_FORMAT_CONVERTER, 7
.equ K15_NODE_METER, 8

# Run the complete K15.8 userspace qualification. This routine is called only
# after the K15.7 command/audio/dead-client/heartbeat proofs have completed and
# immediately before K15.7 asks the kernel to close its aggregate runtime gate.
k15_8_graph_qualify:
    push rbx
    push r12
    push r13
    push r14
    push r15

    call graph_reset_state
    call graph_compile_topology
    test eax, eax
    jnz graph_qualification_fail
    TW_WRITE TW_CONSOLE_HANDLE, graph_compiled_msg, GRAPH_COMPILED_MSG_LEN

    xor r12d, r12d
k15_8_block_loop:
    cmp r12d, K15_GRAPH_BLOCKS
    jae k15_8_blocks_done

    # Qualification source is stereo S16. Block n uses:
    # left  = 1000 + 200*n
    # right = -500 - 100*n
    mov eax, r12d
    imul eax, 200
    add eax, 1000
    mov r13d, eax
    mov eax, r12d
    imul eax, 100
    add eax, 500
    neg eax
    mov r14d, eax

    lea rdi, [rip + graph_input_block]
    mov ecx, K15_GRAPH_FRAMES
k15_8_generate_frames:
    mov word ptr [rdi], r13w
    mov word ptr [rdi + 2], r14w
    add rdi, 4
    dec ecx
    jnz k15_8_generate_frames

    call graph_process_compiled_block
    test eax, eax
    jnz graph_qualification_fail

    # Expected output after Gain(0.5), Splitter, L/R swap, exact format
    # round-trip and Mixer is the same value on both channels:
    # expected = 250 + 50*n.
    mov eax, r12d
    imul eax, 50
    add eax, 250
    mov r15d, eax
    lea rsi, [rip + graph_buffers + (7 * K15_GRAPH_BLOCK_BYTES)]
    mov ecx, K15_GRAPH_SAMPLES
k15_8_verify_output:
    movsx eax, word ptr [rsi]
    cmp eax, r15d
    jne graph_qualification_fail
    add rsi, 2
    dec ecx
    jnz k15_8_verify_output

    inc r12d
    jmp k15_8_block_loop

k15_8_blocks_done:
    cmp dword ptr [rip + graph_blocks_processed], 4
    jne graph_qualification_fail
    cmp dword ptr [rip + graph_frames_processed], 1024
    jne graph_qualification_fail
    cmp dword ptr [rip + graph_format_roundtrips], 4
    jne graph_qualification_fail
    cmp dword ptr [rip + graph_meter_peak], 400
    jne graph_qualification_fail
    cmp qword ptr [rip + graph_meter_sum_abs], 204800
    jne graph_qualification_fail

    lea rsi, [rip + graph_node_runs]
    mov ecx, K15_GRAPH_NODE_COUNT
k15_8_verify_runs:
    cmp dword ptr [rsi], 4
    jne graph_qualification_fail
    add rsi, 4
    dec ecx
    jnz k15_8_verify_runs

    TW_WRITE TW_CONSOLE_HANDLE, graph_nodes_msg, GRAPH_NODES_MSG_LEN
    TW_WRITE TW_CONSOLE_HANDLE, graph_pcm_msg, GRAPH_PCM_MSG_LEN
    TW_WRITE TW_CONSOLE_HANDLE, graph_proof_msg, GRAPH_PROOF_MSG_LEN
    TW_WRITE TW_CONSOLE_HANDLE, graph_ok_msg, GRAPH_OK_MSG_LEN
    TW_WRITE TW_CONSOLE_HANDLE, graph_ready_msg, GRAPH_READY_MSG_LEN
    TW_WRITE TW_CONSOLE_HANDLE, graph_user_ready_msg, GRAPH_USER_READY_MSG_LEN

    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    ret

graph_qualification_fail:
    TW_WRITE TW_CONSOLE_HANDLE, graph_fail_msg, GRAPH_FAIL_MSG_LEN
    jmp fatal

# Clear all compile/runtime qualification state. Fixed graph storage itself is
# static and never allocated at runtime.
graph_reset_state:
    xor eax, eax
    mov dword ptr [rip + graph_edge_count], eax
    mov dword ptr [rip + graph_order_count], eax
    mov dword ptr [rip + graph_compiled], eax
    mov dword ptr [rip + graph_blocks_processed], eax
    mov dword ptr [rip + graph_frames_processed], eax
    mov dword ptr [rip + graph_format_roundtrips], eax
    mov dword ptr [rip + graph_meter_peak], eax
    mov qword ptr [rip + graph_meter_sum_abs], rax

    lea rdi, [rip + graph_indegree]
    mov ecx, K15_GRAPH_NODE_COUNT
    rep stosb
    lea rdi, [rip + graph_emitted]
    mov ecx, K15_GRAPH_NODE_COUNT
    rep stosb
    lea rdi, [rip + graph_order]
    mov al, K15_GRAPH_NODE_NONE
    mov ecx, K15_GRAPH_NODE_COUNT
    rep stosb
    xor eax, eax
    lea rdi, [rip + graph_node_runs]
    mov ecx, K15_GRAPH_NODE_COUNT
    rep stosd
    ret

# Compile the fixed K15.8 DAG using a deterministic Kahn topological sort.
# EAX=0 success, EAX=1 validation/cycle failure.
graph_compile_topology:
    xor r8d, r8d                    # node index
    xor r11d, r11d                  # kind bitset
    xor r12d, r12d                  # edge count

graph_compile_validate_node:
    cmp r8d, K15_GRAPH_NODE_COUNT
    jae graph_compile_validate_done

    lea rbx, [rip + graph_node_kinds]
    movzx eax, byte ptr [rbx + r8]
    cmp eax, K15_NODE_INPUT
    jb graph_compile_fail
    cmp eax, K15_NODE_METER
    ja graph_compile_fail
    dec eax
    bts r11d, eax
    jc graph_compile_fail           # every required kind appears exactly once
    inc eax                         # restore kind value

    xor r9d, r9d                    # required input count
    cmp eax, K15_NODE_INPUT
    je graph_compile_have_required
    mov r9d, 1
    cmp eax, K15_NODE_MIXER
    jne graph_compile_have_required
    mov r9d, 2

graph_compile_have_required:
    mov r10d, r8d
    shl r10d, 2                     # four input slots per node
    xor r13d, r13d                  # slot

graph_compile_input_slot:
    cmp r13d, K15_GRAPH_MAX_INPUTS
    jae graph_compile_next_node
    lea rbx, [rip + graph_inputs]
    mov eax, r10d
    add eax, r13d
    movzx eax, byte ptr [rbx + rax]
    cmp r13d, r9d
    jae graph_compile_unused_slot
    cmp eax, K15_GRAPH_NODE_NONE
    je graph_compile_fail
    cmp eax, K15_GRAPH_NODE_COUNT
    jae graph_compile_fail
    cmp eax, r8d                    # self-edge is immediately invalid
    je graph_compile_fail
    lea rbx, [rip + graph_indegree]
    inc byte ptr [rbx + r8]
    inc r12d
    jmp graph_compile_slot_done

graph_compile_unused_slot:
    cmp eax, K15_GRAPH_NODE_NONE
    jne graph_compile_fail

graph_compile_slot_done:
    inc r13d
    jmp graph_compile_input_slot

graph_compile_next_node:
    inc r8d
    jmp graph_compile_validate_node

graph_compile_validate_done:
    cmp r11d, 0xff
    jne graph_compile_fail
    cmp r12d, K15_GRAPH_EDGE_COUNT
    jne graph_compile_fail
    mov dword ptr [rip + graph_edge_count], r12d

    xor r14d, r14d                  # order count

graph_toposort_outer:
    cmp r14d, K15_GRAPH_NODE_COUNT
    jae graph_toposort_done
    mov r15d, -1                    # selected node
    xor r8d, r8d

graph_toposort_scan:
    cmp r8d, K15_GRAPH_NODE_COUNT
    jae graph_toposort_selected
    lea rbx, [rip + graph_emitted]
    cmp byte ptr [rbx + r8], 0
    jne graph_toposort_scan_next
    lea rbx, [rip + graph_indegree]
    cmp byte ptr [rbx + r8], 0
    jne graph_toposort_scan_next
    mov r15d, r8d                   # deterministic: lowest ready node
    jmp graph_toposort_selected

graph_toposort_scan_next:
    inc r8d
    jmp graph_toposort_scan

graph_toposort_selected:
    cmp r15d, -1
    je graph_compile_fail           # cycle / unresolved dependency
    lea rbx, [rip + graph_emitted]
    mov byte ptr [rbx + r15], 1
    lea rbx, [rip + graph_order]
    mov byte ptr [rbx + r14], r15b
    inc r14d

    # Remove selected node's outgoing edges from remaining indegrees.
    xor r8d, r8d                    # target node

graph_toposort_target:
    cmp r8d, K15_GRAPH_NODE_COUNT
    jae graph_toposort_outer
    lea rbx, [rip + graph_emitted]
    cmp byte ptr [rbx + r8], 0
    jne graph_toposort_target_next
    mov r10d, r8d
    shl r10d, 2
    xor r13d, r13d

graph_toposort_target_slot:
    cmp r13d, K15_GRAPH_MAX_INPUTS
    jae graph_toposort_target_next
    lea rbx, [rip + graph_inputs]
    mov eax, r10d
    add eax, r13d
    movzx eax, byte ptr [rbx + rax]
    cmp eax, K15_GRAPH_NODE_NONE
    je graph_toposort_target_next
    cmp eax, r15d
    jne graph_toposort_target_slot_next
    lea rbx, [rip + graph_indegree]
    cmp byte ptr [rbx + r8], 0
    je graph_compile_fail
    dec byte ptr [rbx + r8]

graph_toposort_target_slot_next:
    inc r13d
    jmp graph_toposort_target_slot

graph_toposort_target_next:
    inc r8d
    jmp graph_toposort_target

graph_toposort_done:
    mov dword ptr [rip + graph_order_count], r14d
    cmp r14d, K15_GRAPH_NODE_COUNT
    jne graph_compile_fail

    # Qualification graph has a unique deterministic execution order 0..7.
    xor r8d, r8d
graph_verify_order:
    cmp r8d, K15_GRAPH_NODE_COUNT
    jae graph_compile_success
    lea rbx, [rip + graph_order]
    movzx eax, byte ptr [rbx + r8]
    cmp eax, r8d
    jne graph_compile_fail
    inc r8d
    jmp graph_verify_order

graph_compile_success:
    mov dword ptr [rip + graph_compiled], 1
    xor eax, eax
    ret

graph_compile_fail:
    mov eax, 1
    ret

# Execute exactly one 1024-byte S16 stereo block through the already-compiled
# plan. No syscalls, allocation, locks, sleep or topology compilation occurs
# anywhere in this routine or its node helpers.
graph_process_compiled_block:
    push r12
    push r13
    push r14
    cmp dword ptr [rip + graph_compiled], 1
    jne graph_process_fail
    cmp dword ptr [rip + graph_order_count], K15_GRAPH_NODE_COUNT
    jne graph_process_fail

    xor r12d, r12d
graph_process_order_loop:
    cmp r12d, K15_GRAPH_NODE_COUNT
    jae graph_process_done
    lea rbx, [rip + graph_order]
    movzx r13d, byte ptr [rbx + r12]
    cmp r13d, K15_GRAPH_NODE_COUNT
    jae graph_process_fail
    lea rbx, [rip + graph_node_kinds]
    movzx r14d, byte ptr [rbx + r13]

    # Per-kind runtime counter; kinds 1..8 map directly to slots 0..7.
    mov eax, r14d
    dec eax
    lea rbx, [rip + graph_node_runs]
    inc dword ptr [rbx + rax * 4]

    cmp r14d, K15_NODE_INPUT
    je graph_node_input
    cmp r14d, K15_NODE_OUTPUT
    je graph_node_output
    cmp r14d, K15_NODE_GAIN
    je graph_node_gain
    cmp r14d, K15_NODE_MIXER
    je graph_node_mixer
    cmp r14d, K15_NODE_SPLITTER
    je graph_node_splitter
    cmp r14d, K15_NODE_CHANNEL_MAPPER
    je graph_node_channel_mapper
    cmp r14d, K15_NODE_FORMAT_CONVERTER
    je graph_node_format_converter
    cmp r14d, K15_NODE_METER
    je graph_node_meter
    jmp graph_process_fail

graph_node_input:
    lea rsi, [rip + graph_input_block]
    mov eax, r13d
    shl eax, 10
    lea rdi, [rip + graph_buffers]
    add rdi, rax
    call graph_copy_block
    jmp graph_process_next

graph_node_output:
    call graph_source0_address
    mov rsi, rax
    mov eax, r13d
    shl eax, 10
    lea rdi, [rip + graph_buffers]
    add rdi, rax
    call graph_copy_block
    jmp graph_process_next

graph_node_gain:
    call graph_source0_address
    mov rsi, rax
    mov eax, r13d
    shl eax, 10
    lea rdi, [rip + graph_buffers]
    add rdi, rax
    mov ecx, K15_GRAPH_SAMPLES
graph_gain_loop:
    movsx eax, word ptr [rsi]
    sar eax, 1                      # exact Q15 0.5 for qualification samples
    mov word ptr [rdi], ax
    add rsi, 2
    add rdi, 2
    dec ecx
    jnz graph_gain_loop
    jmp graph_process_next

graph_node_splitter:
    call graph_source0_address
    mov rsi, rax
    mov eax, r13d
    shl eax, 10
    lea rdi, [rip + graph_buffers]
    add rdi, rax
    call graph_copy_block
    jmp graph_process_next

graph_node_channel_mapper:
    call graph_source0_address
    mov rsi, rax
    mov eax, r13d
    shl eax, 10
    lea rdi, [rip + graph_buffers]
    add rdi, rax
    mov ecx, K15_GRAPH_FRAMES
graph_mapper_loop:
    mov ax, word ptr [rsi]
    mov dx, word ptr [rsi + 2]
    mov word ptr [rdi], dx
    mov word ptr [rdi + 2], ax
    add rsi, 4
    add rdi, 4
    dec ecx
    jnz graph_mapper_loop
    jmp graph_process_next

graph_node_format_converter:
    # S16 stereo interleaved -> planar.
    call graph_source0_address
    mov rsi, rax
    lea rdi, [rip + graph_planar]
    lea rdx, [rip + graph_planar + 512]
    mov ecx, K15_GRAPH_FRAMES
graph_to_planar_loop:
    mov ax, word ptr [rsi]
    mov word ptr [rdi], ax
    mov ax, word ptr [rsi + 2]
    mov word ptr [rdx], ax
    add rsi, 4
    add rdi, 2
    add rdx, 2
    dec ecx
    jnz graph_to_planar_loop

    # Planar -> S16 stereo interleaved, byte/sample exact.
    mov eax, r13d
    shl eax, 10
    lea rdi, [rip + graph_buffers]
    add rdi, rax
    lea rsi, [rip + graph_planar]
    lea rdx, [rip + graph_planar + 512]
    mov ecx, K15_GRAPH_FRAMES
graph_from_planar_loop:
    mov ax, word ptr [rsi]
    mov word ptr [rdi], ax
    mov ax, word ptr [rdx]
    mov word ptr [rdi + 2], ax
    add rsi, 2
    add rdx, 2
    add rdi, 4
    dec ecx
    jnz graph_from_planar_loop
    inc dword ptr [rip + graph_format_roundtrips]
    jmp graph_process_next

graph_node_mixer:
    call graph_source0_address
    mov rsi, rax
    call graph_source1_address
    mov rdx, rax
    mov eax, r13d
    shl eax, 10
    lea rdi, [rip + graph_buffers]
    add rdi, rax
    mov ecx, K15_GRAPH_SAMPLES
graph_mixer_loop:
    movsx eax, word ptr [rsi]
    movsx ebx, word ptr [rdx]
    add eax, ebx
    cmp eax, 32767
    jle graph_mixer_check_low
    mov eax, 32767
    jmp graph_mixer_store
graph_mixer_check_low:
    cmp eax, -32768
    jge graph_mixer_store
    mov eax, -32768
graph_mixer_store:
    mov word ptr [rdi], ax
    add rsi, 2
    add rdx, 2
    add rdi, 2
    dec ecx
    jnz graph_mixer_loop
    jmp graph_process_next

graph_node_meter:
    call graph_source0_address
    mov rsi, rax
    mov eax, r13d
    shl eax, 10
    lea rdi, [rip + graph_buffers]
    add rdi, rax
    xor r8d, r8d                    # peak
    xor r9, r9                      # sum abs
    mov ecx, K15_GRAPH_SAMPLES
graph_meter_loop:
    movsx eax, word ptr [rsi]
    mov word ptr [rdi], ax
    mov edx, eax
    test edx, edx
    jge graph_meter_abs_ready
    neg edx
graph_meter_abs_ready:
    cmp edx, r8d
    jbe graph_meter_peak_done
    mov r8d, edx
graph_meter_peak_done:
    mov edx, edx
    add r9, rdx
    add rsi, 2
    add rdi, 2
    dec ecx
    jnz graph_meter_loop
    mov dword ptr [rip + graph_meter_peak], r8d
    mov qword ptr [rip + graph_meter_sum_abs], r9
    jmp graph_process_next

graph_process_next:
    inc r12d
    jmp graph_process_order_loop

graph_process_done:
    inc dword ptr [rip + graph_blocks_processed]
    add dword ptr [rip + graph_frames_processed], K15_GRAPH_FRAMES
    xor eax, eax
    pop r14
    pop r13
    pop r12
    ret

graph_process_fail:
    mov eax, 1
    pop r14
    pop r13
    pop r12
    ret

# Return source input slot 0 buffer address in RAX for node R13D.
graph_source0_address:
    mov eax, r13d
    shl eax, 2
    lea rbx, [rip + graph_inputs]
    movzx eax, byte ptr [rbx + rax]
    cmp eax, K15_GRAPH_NODE_COUNT
    jae graph_process_fail_source
    shl eax, 10
    lea rbx, [rip + graph_buffers]
    lea rax, [rbx + rax]
    ret

# Return source input slot 1 buffer address in RAX for node R13D.
graph_source1_address:
    mov eax, r13d
    shl eax, 2
    lea rbx, [rip + graph_inputs]
    movzx eax, byte ptr [rbx + rax + 1]
    cmp eax, K15_GRAPH_NODE_COUNT
    jae graph_process_fail_source
    shl eax, 10
    lea rbx, [rip + graph_buffers]
    lea rax, [rbx + rax]
    ret

graph_process_fail_source:
    # Topology compilation guarantees this cannot occur. Return the zero page
    # inside graph storage; caller's qualification checks still fail closed.
    lea rax, [rip + graph_buffers]
    ret

graph_copy_block:
    mov ecx, K15_GRAPH_BLOCK_BYTES / 8
    rep movsq
    ret

.section .rodata
# Node order intentionally differs from kind numeric IDs to prove dispatch is
# driven by compiled topology rather than assuming kind==node index.
graph_node_kinds:
    .byte K15_NODE_INPUT
    .byte K15_NODE_GAIN
    .byte K15_NODE_SPLITTER
    .byte K15_NODE_CHANNEL_MAPPER
    .byte K15_NODE_FORMAT_CONVERTER
    .byte K15_NODE_MIXER
    .byte K15_NODE_METER
    .byte K15_NODE_OUTPUT

# Four source slots per node. 0xff means unused.
# 0 Input: none
# 1 Gain: 0
# 2 Splitter: 1
# 3 ChannelMapper: 2
# 4 FormatConverter: 2
# 5 Mixer: 3,4
# 6 Meter: 5
# 7 Output: 6
graph_inputs:
    .byte 0xff, 0xff, 0xff, 0xff
    .byte 0,    0xff, 0xff, 0xff
    .byte 1,    0xff, 0xff, 0xff
    .byte 2,    0xff, 0xff, 0xff
    .byte 2,    0xff, 0xff, 0xff
    .byte 3,    4,    0xff, 0xff
    .byte 5,    0xff, 0xff, 0xff
    .byte 6,    0xff, 0xff, 0xff

graph_compiled_msg:
    .ascii "[K15GR] graph compiled: generation=1 nodes=8 edges=8 order=8 bounded=true cycle_free=true topology_mutation_rt=false"
.set GRAPH_COMPILED_MSG_LEN, . - graph_compiled_msg
graph_nodes_msg:
    .ascii "[K15GR] node execution: Input=4 Output=4 Gain=4 Mixer=4 Splitter=4 ChannelMapper=4 FormatConverter=4 Meter=4"
.set GRAPH_NODES_MSG_LEN, . - graph_nodes_msg
graph_pcm_msg:
    .ascii "[K15GR] PCM verified: blocks=4 frames=1024 channels=2 format=S16 output_verified=true format_roundtrips=4 meter_peak=400 meter_sum_abs=204800"
.set GRAPH_PCM_MSG_LEN, . - graph_pcm_msg
graph_proof_msg:
    .ascii "[K15GR] ForgeAudio graph proof complete: version=1 generation=1 nodes=8 edges=8 allocation_free=true runtime_locks=0 deterministic_order=true compile_outside_rt=true"
.set GRAPH_PROOF_MSG_LEN, . - graph_proof_msg
graph_ok_msg:
    .ascii "[K15OK] K15.8 ForgeAudio Graph Engine qualified: version=1 generation=1 nodes=8 edges=8 blocks=4 frames=1024 all_nodes=true bounded=true deterministic_order=true"
.set GRAPH_OK_MSG_LEN, . - graph_ok_msg
graph_ready_msg:
    .ascii "[K15GR] ForgeAudio Graph Engine ready: max_nodes=16 max_inputs=4 runtime_locks=0 topology_compile_rt=false sample_accurate_switching=false resampling=false"
.set GRAPH_READY_MSG_LEN, . - graph_ready_msg
graph_user_ready_msg:
    .ascii "[USER] [forgeaudiod] K15.8 graph engine ready: Input=true Output=true Gain=true Mixer=true Splitter=true ChannelMapper=true FormatConverter=true Meter=true"
.set GRAPH_USER_READY_MSG_LEN, . - graph_user_ready_msg
graph_fail_msg:
    .ascii "[FAIL] ForgeAudioD K15.8 Graph Engine qualification failed"
.set GRAPH_FAIL_MSG_LEN, . - graph_fail_msg

.section .bss
.align 64
graph_input_block: .skip K15_GRAPH_BLOCK_BYTES
graph_planar: .skip K15_GRAPH_BLOCK_BYTES
graph_buffers: .skip K15_GRAPH_NODE_COUNT * K15_GRAPH_BLOCK_BYTES
.align 16
graph_indegree: .skip K15_GRAPH_NODE_COUNT
graph_emitted: .skip K15_GRAPH_NODE_COUNT
graph_order: .skip K15_GRAPH_NODE_COUNT
graph_node_runs: .skip K15_GRAPH_NODE_COUNT * 4
graph_edge_count: .skip 4
graph_order_count: .skip 4
graph_compiled: .skip 4
graph_blocks_processed: .skip 4
graph_frames_processed: .skip 4
graph_format_roundtrips: .skip 4
graph_meter_peak: .skip 4
.align 8
graph_meter_sum_abs: .skip 8

.section .text
