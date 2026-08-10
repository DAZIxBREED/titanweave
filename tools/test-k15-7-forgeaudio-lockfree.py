#!/usr/bin/env python3
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
stone = (root / 'K15_STONE_CONTRACT.md').read_text()
status = (root / 'K15_7_SOURCE_STATUS.md').read_text()
k156 = (root / 'K15_6_SOURCE_STATUS.md').read_text()
abi_lib = (root / 'libraries/forgeaudio-abi/src/lib.rs').read_text()
abi = (root / 'kernel/weavecore/src/abi.rs').read_text()
syscalls = (root / 'kernel/weavecore/src/syscalls.rs').read_text()
transport = (root / 'kernel/weavecore/src/forgeaudio_transport.rs').read_text()
process = (root / 'kernel/weavecore/src/process.rs').read_text()
service = (root / 'kernel/weavecore/src/service.rs').read_text()
twabi = (root / 'userspace/include/twabi.inc').read_text()
daemon = (root / 'userspace/forgeaudiod/forgeaudiod.S').read_text()
client = (root / 'userspace/audioclient/audioclient.S').read_text()
builder = (root / 'tools/build-userspace.sh').read_text()
fat = (root / 'tools/make-fat32.py').read_text()
inspector = (root / 'tools/inspect-fat32.py').read_text()
runner = (root / 'tools/run-k15-7-qemu-forgeaudio-lockfree.sh').read_text()
checker = (root / 'tools/check-k15-7-serial-log.sh').read_text()
implementation = (root / 'K15_7_IMPLEMENTATION.md').read_text()
current = (root / 'CURRENT_STATUS.md').read_text()

assert '7. **K15.7 — Lock-Free Audio Transport**' in stone
assert 'Status: **QUALIFIED / FROZEN**' in k156
assert (root / 'K15_6_RUNTIME_QUALIFICATION.md').is_file()
assert 'Status: **SOURCE-INTEGRATED / RUNTIME QUALIFICATION PENDING**' in status
assert 'K15.7 Lock-Free Audio Transport: SOURCE-INTEGRATED / RUNTIME QUALIFICATION PENDING' in current

# Frozen K15.2 object ABI version stays v1; K15.7 has a separate transport ABI.
assert 'FORGEAUDIO_ABI_VERSION: u32 = 1' in abi_lib
for token in [
    'FORGEAUDIO_TRANSPORT_ABI_VERSION: u32 = 1',
    'AUDIO_TRANSPORT_BLOCK_BYTES: usize = 1024',
    'AUDIO_TRANSPORT_RING_SLOTS: usize = 4',
    'AUDIO_TRANSPORT_COMMAND_BYTES: usize = 32',
    'AUDIO_TRANSPORT_COMMAND_DEPTH: usize = 16',
    'AUDIO_TRANSPORT_MAX_SESSIONS: usize = 4',
    'pub struct AudioTransportCommand', 'pub enum AudioTransportOp',
]: assert token in abi_lib, token

# Dedicated syscall 48; frozen K15.2/K15.6 numbers remain unchanged.
assert 'SYS_AUDIO_CONTROL: u64 = 46' in abi
assert 'SYS_AUDIO_SERVER_CONTROL: u64 = 47' in abi
assert 'SYS_AUDIO_TRANSPORT_CONTROL: u64 = 48' in abi
for token in ['TW_SYS_AUDIO_TRANSPORT_CONTROL, 48', 'TW_AUDIO_TRANSPORT_CLIENT_ATTACH, 1',
              'TW_AUDIO_TRANSPORT_SERVER_QUALIFY, 13']:
    assert token in twabi, token
for token in ['SYS_AUDIO_TRANSPORT_CONTROL =>', 'syscall_audio_transport_control',
              'AudioTransportOp::ClientAttach', 'AudioTransportOp::ServerQualify',
              'current_copy_from_user', 'current_copy_to_user']:
    assert token in syscalls, token

# Hot-path queues must be fixed-size SPSC atomics, no locks/allocations.
for token in ['AtomicU32', 'AtomicU64', 'Ordering::Acquire', 'Ordering::Release',
              'struct BlockRing', 'struct CommandRing', 'head: AtomicU64', 'tail: AtomicU64',
              'AUDIO_TRANSPORT_RING_SLOTS', 'AUDIO_TRANSPORT_COMMAND_DEPTH',
              'client_push_playback', 'server_pop_playback', 'server_push_capture', 'client_pop_capture',
              'client_push_command', 'server_pop_command', 'server_push_command', 'client_pop_command',
              'detach_process', 'reap_dead', 'qualification_snapshot',
              '[K15LF] transport attached:', '[K15LF] dead client isolated:']:
    assert token in transport, token
for forbidden in ['SpinLock', 'Mutex', 'Vec<', 'Box<', 'alloc::', 'sleep(', 'todo!()', 'unimplemented!()']:
    assert forbidden not in transport, forbidden

# Dead-client isolation advances generation, resets only the session and keeps daemon alive.
for token in ['SESSION_DEAD', 'generation.fetch_add(1', 'session.reset_rings()',
              'DEAD_CLIENTS.fetch_add', 'GENERATION_ADVANCES.fetch_add',
              'STALE_REJECTIONS.fetch_add']:
    assert token in transport, token
for token in ['crate::forgeaudio_transport::detach_process(pid);',
              'forgeaudio_transport_ready', 'forgeaudio_transport_dead_isolated',
              'note_forgeaudio_transport_ready', 'forgeaudiod_server_pid_if_ready']:
    assert token in process, token

# Real second userspace process, after daemon and before shell.
for token in ['AudioClient,', 'SERVICE_SPECS: [ServiceSpec; 10]', 'AUDIOCLT.ELF',
              'process_name: b"audio-client"', 'ServiceRole::AudioClient']:
    assert token in service, token
assert service.index('AUDIOD.ELF') < service.index('AUDIOCLT.ELF') < service.index('SHELL.ELF')
for token in ['audioclient', 'AUDIOCLT']:
    assert token in builder, token
for token in ['AUDIOCLT.ELF', 'audio-client=C:\\\\SYSTEM\\\\SERVICES\\\\AUDIOCLT.ELF']:
    assert token in fat, token
assert "'AUDIOCLT.ELF'" in inspector

# Application-side stress: depth-16 commands and 3x4-slot audio laps.
for token in ['TW_AUDIO_TRANSPORT_CLIENT_ATTACH', 'TW_AUDIO_TRANSPORT_CLIENT_PUSH_COMMAND',
              'TW_AUDIO_TRANSPORT_CLIENT_POP_COMMAND', 'TW_AUDIO_TRANSPORT_CLIENT_PUSH_PLAYBACK',
              'TW_AUDIO_TRANSPORT_CLIENT_POP_CAPTURE', 'TW_AUDIO_TRANSPORT_COMMAND_DEPTH + 1',
              'cmp r13d, 3', 'TW_AUDIO_TRANSPORT_RING_SLOTS', 'ERROR_WOULD_BLOCK',
              'TW_EXIT 0', 'data_verified=true', 'dead-client isolation']:
    assert token in client, token

# Server consumes/returns exact data, rejects stale generation, reaps and survives.
for token in ['TW_AUDIO_TRANSPORT_SERVER_FIND_ACTIVE', 'TW_AUDIO_TRANSPORT_SERVER_POP_COMMAND',
              'TW_AUDIO_TRANSPORT_SERVER_PUSH_COMMAND', 'TW_AUDIO_TRANSPORT_SERVER_POP_PLAYBACK',
              'TW_AUDIO_TRANSPORT_SERVER_PUSH_CAPTURE', 'TW_AUDIO_TRANSPORT_SERVER_FIND_DEAD',
              'TW_AUDIO_TRANSPORT_SERVER_REAP_DEAD', 'TW_AUDIO_TRANSPORT_SERVER_QUALIFY',
              'transport_new_generation', 'cmp rax, ERROR_INVALID_STATE',
              'mov rsi, 2', 'post-isolation heartbeat', 'no_DSP=true']:
    assert token in daemon, token
assert daemon.index('transport_dead_msg') < daemon.index('mov rsi, 2') < daemon.index('TW_AUDIO_TRANSPORT_SERVER_QUALIFY')

# No K15.8 graph/mixer or later resampler work may appear.
for forbidden in ['forgeaudio_graph.rs', 'MixerNode', 'GainNode', 'Resampler', 'SampleAccurateSwitch']:
    assert forbidden not in transport, forbidden
    assert forbidden not in client, forbidden
assert not (root / 'kernel/weavecore/src/forgeaudio_graph.rs').exists()
assert not (root / 'kernel/weavecore/src/forgeaudio_resampler.rs').exists()

# Every userspace message stays within K6's 256-byte write ABI.
for text in (daemon, client):
    for literal in re.findall(r'\.ascii "([^"]*)"', text):
        assert len(literal.encode()) <= 256, (len(literal.encode()), literal)

for rel in [
    'kernel/weavecore/src/forgeaudio_transport.rs', 'userspace/audioclient/audioclient.S',
    'K15_7_IMPLEMENTATION.md', 'K15_7_SOURCE_STATUS.md', 'K15_7_TESTER_GUIDE.md',
    'tools/run-k15-7-qemu-forgeaudio-lockfree.sh', 'tools/check-k15-7-serial-log.sh',
]: assert (root / rel).is_file(), rel

for token in ['ich9-intel-hda', 'hda-duplex', 'K15.7 ForgeAudio Lock-Free Transport QEMU qualification',
              'check-k15-7-serial-log.sh']:
    assert token in runner, token
for token in ['inherited K15.1-K15.6 ForgeAudio qualification retained',
              '[K15LF] transport attached:', '[K15LF] dead client isolated:',
              '[K15OK] K15.7 ForgeAudio lock-free transport qualified:',
              '[K15LR] ForgeAudio lock-free transport ready:']:
    assert token in checker, token
assert 'TITANWEAVE_ALLOW_LATER_GATE_FAILURES=1' in checker
assert 'bounded application/server audio rings' in implementation
assert 'K15.8' in implementation

print('Titanweave K15.7 ForgeAudio lock-free transport source checks passed.')
