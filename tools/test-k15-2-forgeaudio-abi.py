#!/usr/bin/env python3
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
abi = (root / 'libraries/forgeaudio-abi/src/lib.rs').read_text()
kernel = (root / 'kernel/weavecore/src/forgeaudio.rs').read_text()
syscalls = (root / 'kernel/weavecore/src/syscalls.rs').read_text()
handles = (root / 'kernel/weavecore/src/handles.rs').read_text()
process = (root / 'kernel/weavecore/src/process.rs').read_text()
main = (root / 'kernel/weavecore/src/main.rs').read_text()
user = (root / 'libraries/user-runtime/src/lib.rs').read_text()
asm = (root / 'userspace/include/twabi.inc').read_text()
workspace = (root / 'Cargo.toml').read_text()
stone = (root / 'K15_STONE_CONTRACT.md').read_text()
implementation = (root / 'K15_2_IMPLEMENTATION.md').read_text()

for token in [
    'FORGEAUDIO_ABI_VERSION: u32 = 1',
    'AudioObjectKind',
    'AudioDeviceInfo',
    'AudioEndpointInfo',
    'AudioStreamConfig',
    'AudioBufferInfo',
    'AudioClockSnapshot',
    'AudioEventRecord',
    'AudioFenceInfo',
    'AudioControlRequest',
    'AudioControlResponse',
    'AudioControlOp',
    'const _: [(); 72] = [(); core::mem::size_of::<AudioDeviceInfo>()]',
    'const _: [(); 64] = [(); core::mem::size_of::<AudioControlResponse>()]',
]:
    assert token in abi, token

for token in [
    'MAX_AUDIO_DEVICES: usize = 16',
    'MAX_AUDIO_ENDPOINTS: usize = 64',
    'MAX_AUDIO_STREAMS: usize = 64',
    'MAX_AUDIO_BUFFERS: usize = 32',
    'MAX_ABI_BUFFER_BYTES: usize = 16 * 1024',
    'register_device',
    'register_endpoint',
    'open_device',
    'open_stream',
    'configure_stream',
    'prepare_stream',
    'start_stream',
    'drain_stream',
    'stop_stream',
    'recover_stream',
    'create_buffer',
    'write_buffer',
    'read_buffer',
    'create_clock',
    'clock_snapshot',
    'create_event',
    'push_event',
    'poll_event',
    'create_fence',
    'signal_fence',
    'release_object',
    'run_abi_self_test',
    'K15.2 must not invent an audio hardware device',
    'fake_devices=false',
    '[K15OK] K15.2 ForgeAudio kernel ABI qualified:',
]:
    assert token in kernel, token

# The qualification path must not register invented hardware.
self_test = kernel.split('pub fn run_abi_self_test()', 1)[1]
assert 'register_device(' not in self_test
assert 'register_endpoint(' not in self_test
assert 'device_count() != 0' in self_test
assert 'open_device(0xDEAD_BEEF).is_ok()' in self_test

for token in [
    'SYS_AUDIO_ABI_QUERY: u64 = 44',
    'SYS_AUDIO_ENUMERATE: u64 = 45',
    'SYS_AUDIO_CONTROL: u64 = 46',
]:
    assert token in (root / 'kernel/weavecore/src/abi.rs').read_text(), token

for token in [
    'SYS_AUDIO_ABI_QUERY =>',
    'SYS_AUDIO_ENUMERATE =>',
    'SYS_AUDIO_CONTROL =>',
    'syscall_audio_abi_query',
    'syscall_audio_enumerate',
    'syscall_audio_control',
    'AudioControlOp::OpenDevice',
    'AudioControlOp::OpenStream',
    'AudioControlOp::ConfigureStream',
    'AudioControlOp::PrepareStream',
    'AudioControlOp::StartStream',
    'AudioControlOp::StopStream',
    'AudioControlOp::DrainStream',
    'AudioControlOp::RecoverStream',
    'AudioControlOp::QueryPosition',
    'AudioControlOp::CreateBuffer',
    'AudioControlOp::CreateClock',
    'AudioControlOp::CreateEvent',
    'AudioControlOp::CreateFence',
    'AudioControlOp::PollEvent',
    'AudioControlOp::QueryFence',
    'AudioControlOp::CloseObject',
]:
    assert token in syscalls, token

assert 'Audio { object_id: u64, kind: AudioObjectKind }' in handles
assert 'current_allocate_handle' in process
assert 'current_close_handle' in process
assert 'crate::forgeaudio::release_object' in process

for token in [
    'pub use titanweave_forgeaudio_abi::*;',
    'SYS_AUDIO_ABI_QUERY: u64 = 44',
    'SYS_AUDIO_ENUMERATE: u64 = 45',
    'SYS_AUDIO_CONTROL: u64 = 46',
    'audio_abi_query',
    'audio_enumerate_device',
    'audio_enumerate_endpoint',
    'audio_control',
]:
    assert token in user, token
for token in [
    '.equ TW_SYS_AUDIO_ABI_QUERY, 44',
    '.equ TW_SYS_AUDIO_ENUMERATE, 45',
    '.equ TW_SYS_AUDIO_CONTROL, 46',
]:
    assert token in asm, token

assert '"libraries/forgeaudio-abi"' in workspace
assert 'mod forgeaudio;' in main
assert 'forgeaudio::initialize()' in main
assert 'forgeaudio::run_abi_self_test()' in main
assert '[K15ARD] ForgeAudio ABI ready:' in main

# Stone contract remains exactly 16 gates.
gates = re.findall(r'^\d+\. \*\*K15\.(\d+) — ', stone, flags=re.MULTILINE)
assert gates == [str(i) for i in range(1, 17)], gates
assert 'K15 ends at K15.16.' in stone

# Required K15.2 source paths must not contain implementation placeholders.
for path, text in [
    ('forgeaudio.rs', kernel),
    ('forgeaudio-abi/lib.rs', abi),
    ('syscalls.rs audio section', syscalls[syscalls.index('fn syscall_audio_abi_query'):]),
]:
    lowered = text.lower()
    for forbidden in ['todo!', 'unimplemented!', 'fake success', 'placeholder']:
        assert forbidden not in lowered, f'{path}: forbidden token {forbidden!r}'

assert 'shared' in implementation.lower() and 'syscall' in implementation.lower()
print('Titanweave K15.2 ForgeAudio kernel ABI source checks passed.')
