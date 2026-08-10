#!/usr/bin/env python3
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
stone = (root / 'K15_STONE_CONTRACT.md').read_text()
graph = (root / 'kernel/weavecore/src/forgeaudio_graph_engine.rs').read_text()
main = (root / 'kernel/weavecore/src/main.rs').read_text()
syscalls = (root / 'kernel/weavecore/src/syscalls.rs').read_text()
daemon = (root / 'userspace/forgeaudiod/forgeaudiod.S').read_text()
runner = (root / 'tools/run-k15-8-qemu-forgeaudio-graph.sh').read_text()
checker = (root / 'tools/check-k15-8-serial-log.sh').read_text()

assert '8. **K15.8 — ForgeAudio Graph Engine**' in stone
assert 'mod forgeaudio_graph_engine;' in main

for token in [
    'FORGEAUDIO_GRAPH_ENGINE_VERSION: u32 = 1',
    'MAX_GRAPH_NODES: usize = 16', 'MAX_GRAPH_INPUTS: usize = 4',
    'Input = 1', 'Output = 2', 'Gain = 3', 'Mixer = 4',
    'Splitter = 5', 'ChannelMapper = 6', 'FormatConverter = 7', 'Meter = 8',
    'fn add_node', 'fn connect', 'fn compile', 'fn process_block',
    'graph contains a cycle', 'graph topology is immutable after compile',
    'run_qualification', '[K15GR] graph compiled:', '[K15GR] node execution:',
    '[K15GR] PCM verified:', '[K15GR] ForgeAudio graph proof complete:',
]:
    assert token in graph, token

# Required processing path is fixed-capacity and contains no allocator/lock/sleep primitive.
for forbidden in ['Vec<', 'Box<', 'alloc::', 'Mutex', 'SpinLock', 'sleep(',
                  'todo!()', 'unimplemented!()', 'File::']:
    assert forbidden not in graph, forbidden

# K15.8 executes only after K15.7 ServerQualify succeeds, so a graph failure
# fails ForgeAudioD closed and prevents the userspace qualification halt.
assert 'crate::forgeaudio_graph_engine::run_qualification()' in syscalls
for token in [
    '[K15OK] K15.7 ForgeAudio lock-free transport qualified:',
    '[K15OK] K15.8 ForgeAudio Graph Engine qualified:',
    '[K15GR] ForgeAudio Graph Engine ready:',
    'sample_accurate_switching=false', 'resampling=false',
]:
    assert token in syscalls, token
assert syscalls.index('[K15OK] K15.7 ForgeAudio lock-free transport qualified:') < syscalls.index('crate::forgeaudio_graph_engine::run_qualification()')

assert '[USER] [forgeaudiod] K15.8 graph engine ready:' in daemon
for token in ['Input=true', 'Output=true', 'Gain=true', 'Mixer=true', 'Splitter=true',
              'ChannelMapper=true', 'FormatConverter=true', 'Meter=true']:
    assert token in daemon, token

# Do not steal K15.9/K15.11 scope.
for forbidden in ['SampleAccurateSwitch', 'activate_at_frame', 'ResamplerNode', 'fractional_drift']:
    assert forbidden not in graph, forbidden
    assert forbidden not in daemon, forbidden

# Every userspace console literal remains inside the K6 256-byte write ABI.
for literal in re.findall(r'\.ascii "([^"]*)"', daemon):
    assert len(literal.encode()) <= 256, (len(literal.encode()), literal)

for token in ['K15.8 ForgeAudio Graph Engine QEMU qualification',
              'run-k15-7-qemu-forgeaudio-lockfree.sh', 'check-k15-8-serial-log.sh']:
    assert token in runner, token
for token in ['inherited K15.1-K15.7 ForgeAudio qualification retained',
              '[K15GR] graph compiled:', '[K15OK] K15.8 ForgeAudio Graph Engine qualified:',
              'Titanweave K15.8 ForgeAudio Graph Engine runtime qualification PASSED.']:
    assert token in checker, token

print('Titanweave K15.8 ForgeAudio Graph Engine source checks passed.')
