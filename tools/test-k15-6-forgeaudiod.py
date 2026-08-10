#!/usr/bin/env python3
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
stone = (root / 'K15_STONE_CONTRACT.md').read_text()
status = (root / 'K15_6_SOURCE_STATUS.md').read_text()
k155_status = (root / 'K15_5_SOURCE_STATUS.md').read_text()
service = (root / 'kernel/weavecore/src/service.rs').read_text()
process = (root / 'kernel/weavecore/src/process.rs').read_text()
forgeaudio = (root / 'kernel/weavecore/src/forgeaudio.rs').read_text()
abi = (root / 'kernel/weavecore/src/abi.rs').read_text()
syscalls = (root / 'kernel/weavecore/src/syscalls.rs').read_text()
twabi = (root / 'userspace/include/twabi.inc').read_text()
daemon = (root / 'userspace/forgeaudiod/forgeaudiod.S').read_text()
builder = (root / 'tools/build-userspace.sh').read_text()
fat = (root / 'tools/make-fat32.py').read_text()
inspector = (root / 'tools/inspect-fat32.py').read_text()
runner = (root / 'tools/run-k15-6-qemu-forgeaudiod.sh').read_text()
checker = (root / 'tools/check-k15-6-serial-log.sh').read_text()
implementation = (root / 'K15_6_IMPLEMENTATION.md').read_text()
current = (root / 'CURRENT_STATUS.md').read_text()

assert '6. **K15.6 — ForgeAudioD**' in stone
assert 'Status: **QUALIFIED / FROZEN**' in k155_status
assert (root / 'K15_5_RUNTIME_QUALIFICATION.md').is_file()
assert 'Status: **SOURCE-INTEGRATED / RUNTIME QUALIFICATION PENDING**' in status
assert 'K15.6 ForgeAudioD: SOURCE-INTEGRATED / RUNTIME QUALIFICATION PENDING' in current

# Real ninth boot service, before shell qualification can finish.
for token in ['Audio,', 'SERVICE_SPECS: [ServiceSpec; 9]', 'AUDIOD.ELF', 'process_name: b"forgeaudiod"', 'ServiceRole::Audio']:
    assert token in service, token
assert service.index('AUDIOD.ELF') < service.index('SHELL.ELF')
for token in ['forgeaudiod', 'AUDIOD']:
    assert token in builder, token
for token in ['AUDIOD.ELF', 'audio=C:\\\\SYSTEM\\\\SERVICES\\\\AUDIOD.ELF']:
    assert token in fat, token
assert "'AUDIOD.ELF'" in inspector

# Dedicated bounded server-control syscall. Existing K15.2 audio syscalls stay 44-46.
assert 'SYS_AUDIO_CONTROL: u64 = 46' in abi
assert 'SYS_AUDIO_SERVER_CONTROL: u64 = 47' in abi
assert '.equ TW_SYS_AUDIO_SERVER_CONTROL, 47' in twabi
for token in ['TW_AUDIO_SERVER_OP_REGISTER, 1', 'TW_AUDIO_SERVER_OP_PUBLISH, 2', 'TW_AUDIO_SERVER_OP_HEARTBEAT, 3']:
    assert token in twabi, token
for token in [
    'SYS_AUDIO_SERVER_CONTROL =>', 'syscall_audio_server_control',
    'const REGISTER: u64 = 1', 'const PUBLISH: u64 = 2', 'const HEARTBEAT: u64 = 3',
    'route_count != 2', 'graph_generation == 0', 'recovery_count == 0',
    'snapshot.streams < 2', 'snapshot.playback_streams == 0',
    'snapshot.capture_streams == 0', 'snapshot.prepared_streams < 2',
    'snapshot.buffers < 2', 'snapshot.clocks == 0', 'snapshot.events == 0', 'snapshot.fences == 0',
    '[K15D] ForgeAudioD ownership verified:',
]:
    assert token in syscalls, token

# Kernel independently derives live process ownership from frozen ForgeAudio tables.
for token in [
    'ForgeAudioServerOwnershipSnapshot', 'server_ownership_snapshot',
    'slot.owner_pid == owner_pid', 'slot.device_object_id == device_object_id',
    'playback_streams', 'capture_streams', 'prepared_streams',
]:
    assert token in forgeaudio, token

# K6 conditional requirement preserves no-HDA historical runs but makes HDA boots
# wait for the real server and a post-yield heartbeat.
for token in [
    'current_service_role() -> ServiceRole', 'register_forgeaudiod',
    'note_forgeaudiod_ready', 'note_forgeaudiod_heartbeat',
    'audio_server_required = crate::forgeaudio::device_count() != 0',
    'forgeaudiod_ready || !runtime.forgeaudiod_heartbeat',
    'forgeaudiod_failed', 'heartbeat sequence must advance',
    '[K15OK] K15.6 ForgeAudioD userspace audio server qualified:',
    '[K15SR] ForgeAudioD ready:',
    '[K15D] required ForgeAudioD ready+heartbeat milestones complete; userspace qualification may close',
]:
    assert token in process, token

# Userspace daemon must exercise real ABI objects and stay persistent.
for token in [
    'TW_SYS_AUDIO_ABI_QUERY', 'TW_SYS_AUDIO_ENUMERATE', 'AUDIO_BACKEND_HDA',
    'device_info + 27', 'device_info + 28',
    'TW_AUDIO_SERVER_OP_REGISTER', 'TW_AUDIO_SERVER_OP_PUBLISH', 'TW_AUDIO_SERVER_OP_HEARTBEAT',
    'TW_AUDIO_OP_OPEN_DEVICE', 'TW_AUDIO_OP_OPEN_STREAM', 'TW_AUDIO_OP_CONFIGURE_STREAM',
    'TW_AUDIO_OP_PREPARE_STREAM', 'TW_AUDIO_OP_CREATE_BUFFER', 'TW_AUDIO_OP_CREATE_CLOCK',
    'TW_AUDIO_OP_CREATE_EVENT', 'TW_AUDIO_OP_CREATE_FENCE', 'TW_AUDIO_OP_QUERY_POSITION',
    'TW_AUDIO_OP_POLL_EVENT', 'TW_AUDIO_OP_QUERY_FENCE', 'TW_AUDIO_OP_CLOSE_OBJECT',
    'ERROR_INVALID_STATE', 'PCM_RATE, 48000', 'PCM_PERIOD_FRAMES, 256',
    'PCM_BUFFER_FRAMES, 1024', 'PCM_BUFFER_BYTES, 4096',
    'route_table: .zero 32', 'service_loop:', 'TW_SYS_YIELD',
    'no_mixing=true', 'no_resampling=true',
    '[USER] [forgeaudiod] K15.6 telemetry:',
    '[USER] [forgeaudiod] K15.6 ForgeAudioD ready:',
    '[USER] [forgeaudiod] K15.6 persistent heartbeat:',
]:
    assert token in daemon, token
assert daemon.index('TW_AUDIO_SERVER_OP_PUBLISH') < daemon.index('mov rax, TW_SYS_YIELD') < daemon.index('TW_AUDIO_SERVER_OP_HEARTBEAT')
assert daemon.count('TW_AUDIO_OP_CREATE_BUFFER') >= 1
assert 'cmp rax, ERROR_INVALID_STATE' in daemon
assert 'stream_rebuilt=true' in daemon
assert 'TW_EXIT 0' not in daemon  # no success-by-exit shortcut

# All visible user writes must fit the K6 256-byte SYS_WRITE contract.
for literal in re.findall(r'\.ascii "([^"]*)"', daemon):
    assert len(literal.encode()) <= 256, (len(literal.encode()), literal)

# K15.6 is control-plane only; later locked gates stay absent.
for forbidden in ['LockFreeAudioRing', 'ClientAudioRing', 'GraphEngine', 'MixerNode', 'Resampler', 'SampleAccurateSwitch']:
    assert forbidden not in daemon, forbidden
    assert forbidden not in syscalls, forbidden
assert not (root / 'kernel/weavecore/src/forgeaudio_graph.rs').exists()
assert not (root / 'kernel/weavecore/src/forgeaudio_resampler.rs').exists()

for rel in [
    'userspace/forgeaudiod/forgeaudiod.S', 'K15_6_IMPLEMENTATION.md',
    'K15_6_SOURCE_STATUS.md', 'K15_6_TESTER_GUIDE.md',
    'tools/run-k15-6-qemu-forgeaudiod.sh', 'tools/check-k15-6-serial-log.sh',
]:
    assert (root / rel).is_file(), rel

for forbidden in ['todo!()', 'unimplemented!()', 'panic!("TODO']:
    assert forbidden not in syscalls, forbidden
    assert forbidden not in process, forbidden

for token in [
    'ich9-intel-hda', 'hda-duplex', 'K15.6 ForgeAudioD QEMU qualification',
    'check-k15-6-serial-log.sh', 'K15.7 Lock-Free Audio Transport remains locked',
]:
    assert token in runner, token
for token in [
    'inherited K15.1-K15.5 ForgeAudio qualification retained',
    '[K15D] ForgeAudioD registered:', '[K15D] ForgeAudioD ownership verified:',
    '[K15D] ForgeAudioD heartbeat:',
    '[K15OK] K15.6 ForgeAudioD userspace audio server qualified:',
    '[K15SR] ForgeAudioD ready:',
    '[K15D] required ForgeAudioD ready+heartbeat milestones complete; userspace qualification may close',
]:
    assert token in checker, token
assert 'TITANWEAVE_ALLOW_LATER_GATE_FAILURES=1' in checker
assert 'control-plane server' in implementation
assert 'K15.7' in implementation and 'K15.8' in implementation

print('Titanweave K15.6 ForgeAudioD source checks passed.')
