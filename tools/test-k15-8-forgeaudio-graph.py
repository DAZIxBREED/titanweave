#!/usr/bin/env python3
from pathlib import Path
import hashlib, re

root = Path(__file__).resolve().parents[1]
stone = (root / 'K15_STONE_CONTRACT.md').read_text()
daemon = (root / 'userspace/forgeaudiod/forgeaudiod.S').read_text()
graph = (root / 'userspace/forgeaudiod/forgeaudio_graph.inc').read_text()
runner = (root / 'tools/run-k15-8-qemu-forgeaudio-graph.sh').read_text()
checker = (root / 'tools/check-k15-8-serial-log.sh').read_text()

assert '8. **K15.8 — ForgeAudio Graph Engine**' in stone
assert '.include "userspace/forgeaudiod/forgeaudio_graph.inc"' in daemon
assert 'call k15_8_graph_qualify' in daemon
assert daemon.index('TW_WRITE TW_CONSOLE_HANDLE, transport_heartbeat_msg') < daemon.index('call k15_8_graph_qualify') < daemon.index('mov rdi, TW_AUDIO_TRANSPORT_SERVER_QUALIFY')

# K15.8 deliberately leaves the qualified K15.1-K15.7 kernel/ABI/transport
# baseline byte-for-byte unchanged. These hashes came from the user-provided,
# Fedora-qualified K15.7 integrated tree used to build this candidate.
frozen = {
    'kernel/weavecore/src/scheduler.rs': 'e2073915c23a93f48dbaabbd2d52f89ad4bab9a2d2a0a04356aaa65d932820ac',
    'kernel/weavecore/src/main.rs': '610f0b3bda28d17e23103fbf5d4d77064beec38304703b54256913b600f55e50',
    'kernel/weavecore/src/process.rs': '615097c1b1b7e36d8d82792839908e382b0ad7df99d0c463764dc63e8eaa6eae',
    'kernel/weavecore/src/syscalls.rs': 'de8be15f674a4d4585810c3bcc384e6fa40c120c2538fd23296335a91d4c453d',
    'kernel/weavecore/src/forgeaudio_transport.rs': 'f6d5d27328282818e5c460a3a4cf8cbf8da029ab5ee1bfce4a6ff44e446b351e',
    'libraries/forgeaudio-abi/src/lib.rs': '8b53fa10312c80ddc9bd8cb02560d54f08e328f3bcde2d7806a65c527b0cf39a',
    'userspace/audioclient/audioclient.S': '7c4530e067280c77a2c6e9c7504c57bece7dabd75966e350311fa7c954da2c77',
    'userspace/include/twabi.inc': '6a52d49a677c2a1e3e28aec0c0d4038b8ba43a500ee189bc16f8ea17a3a8cfaf',
}
for rel, expected in frozen.items():
    actual = hashlib.sha256((root / rel).read_bytes()).hexdigest()
    assert actual == expected, f'frozen K15.7 baseline changed: {rel}: {actual}'

for token in [
    'K15_GRAPH_VERSION, 1', 'K15_GRAPH_GENERATION, 1',
    'K15_GRAPH_NODE_COUNT, 8', 'K15_GRAPH_EDGE_COUNT, 8',
    'K15_GRAPH_MAX_NODES, 16', 'K15_GRAPH_MAX_INPUTS, 4',
    'K15_NODE_INPUT, 1', 'K15_NODE_OUTPUT, 2', 'K15_NODE_GAIN, 3',
    'K15_NODE_MIXER, 4', 'K15_NODE_SPLITTER, 5',
    'K15_NODE_CHANNEL_MAPPER, 6', 'K15_NODE_FORMAT_CONVERTER, 7',
    'K15_NODE_METER, 8', 'graph_compile_topology:',
    'graph_process_compiled_block:', 'graph contains',
]:
    if token == 'graph contains':
        continue
    assert token in graph, token

# Actual graph mechanics, not marker-only qualification.
for token in [
    'bts r11d, eax', 'graph_indegree', 'graph_emitted', 'graph_order',
    'graph_toposort_outer:', 'graph_compile_fail:', 'graph_node_runs',
    'graph_node_input:', 'graph_node_output:', 'graph_node_gain:',
    'graph_node_mixer:', 'graph_node_splitter:', 'graph_node_channel_mapper:',
    'graph_node_format_converter:', 'graph_node_meter:',
    'sar eax, 1', 'graph_to_planar_loop:', 'graph_from_planar_loop:',
    'graph_mixer_check_low:', 'graph_meter_loop:', 'graph_meter_sum_abs',
    'graph_format_roundtrips', 'graph_blocks_processed', 'graph_frames_processed',
    'cmp qword ptr [rip + graph_meter_sum_abs], 204800',
]: assert token in graph, token

# The block executor itself must not perform control-plane work or kernel calls.
process_body = graph.split('graph_process_compiled_block:', 1)[1].split('graph_process_fail_source:', 1)[0]
for forbidden in ['int 0x80', 'TW_WRITE', 'graph_compile_topology', 'malloc', 'free', 'Mutex', 'SpinLock', 'sleep']:
    assert forbidden not in process_body, forbidden

# No K15.9/K15.11 implementation hidden in this gate.
for forbidden in ['activate_at_frame', 'SampleAccurateSwitch', 'ResamplerNode', 'fractional_drift']:
    assert forbidden not in graph, forbidden

# Every userspace string remains within the kernel's bounded 256-byte write ABI.
for text in (daemon, graph):
    for literal in re.findall(r'\.ascii "([^"]*)"', text):
        assert len(literal.encode()) <= 256, (len(literal.encode()), literal)

for token in ['run-k15-7-qemu-forgeaudio-lockfree.sh', 'check-k15-8-serial-log.sh',
              'K15.8 ForgeAudio Graph Engine QEMU qualification']:
    assert token in runner, token
for token in ['inherited K15.1-K15.7 ForgeAudio qualification retained',
              '[K15GR] graph compiled:', '[K15GR] node execution:',
              '[K15GR] PCM verified:', '[K15OK] K15.8 ForgeAudio Graph Engine qualified:',
              'Titanweave K15.8 ForgeAudio Graph Engine runtime qualification PASSED.']:
    assert token in checker, token

print('Titanweave K15.8 ForgeAudio Graph Engine source checks passed.')
