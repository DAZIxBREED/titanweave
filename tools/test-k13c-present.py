#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
def text(path: str) -> str:
    return (root / path).read_text()

policy = text('kernel/weavecore/src/gpu_present.rs')
for token in [
    'K13.C compositor-presentation policy', 'PRESENT_BUFFER_COUNT: usize = 3',
    'MAX_IN_FLIGHT_FRAMES', 'DamageRect', 'byte_offset', 'FramePacer',
    'PresentWatchdog', 'PRESENT_STALL_LIMIT', 'run_self_test',
]:
    assert token in policy, token

virtio = text('kernel/weavecore/src/virtio_gpu.rs')
for token in [
    'LIVE_TRANSPORT', 'VirtioGpuPresentationReport', 'VirtioGpuPresentResult',
    'VIRTIO_GPU_FLAG_FENCE', 'initialize_presentation', 'present_compositor_frame',
    'flush_fenced', 'fence completion did not match submitted fence',
    'render_compositor_damage', 'disable_accelerated_presentation', 'PRESENT_BUFFER_COUNT', 'DamageRect',
    'transfer_to_host(buffer.resource_id, damage)',
    'set_scanout(buffer.resource_id, self.scanout_id',
]:
    assert token in virtio, token

# K13.C must build on K13.B's claimed live transport rather than rediscovering
# or enabling a second unowned GPU path.
present_start = virtio.index('pub fn initialize_presentation(')
present_end = virtio.index('pub fn present_compositor_frame', present_start)
present_block = virtio[present_start:present_end]
assert 'claim_pci_function' not in present_block
assert 'enable_bus_master' not in present_block
assert 'LIVE_TRANSPORT.lock()' in present_block
assert 'forgebus::allocate_dma' in present_block

runtime = text('kernel/weavecore/src/gpu_runtime.rs')
for token in [
    'initialize_presentation', '[PRES]', '[DMG ]', '[PFEN]', '[FBCK]', '[GCOMP]',
    'present_from_displayd', '[UPRS]', 'presentation_ready: true', 'accelerated GPU path fenced after presentation failure',
]:
    assert token in runtime, token

main = text('kernel/weavecore/src/main.rs')
for token in ['gpu_runtime::initialize_presentation(&mut allocator)', '[GPRE] K13.C buffered presentation ready:']:
    assert token in main, token

abi = text('kernel/weavecore/src/abi.rs')
assert 'SYS_GPU_PRESENT: u64 = 9' in abi
syscalls = text('kernel/weavecore/src/syscalls.rs')
for token in ['SYS_GPU_PRESENT', 'GRAPHICS_PRESENT_OBJECT_ID', 'gpu_runtime::present_from_displayd']:
    assert token in syscalls, token
handles = text('kernel/weavecore/src/handles.rs')
for token in ['DISPLAY_PRESENT_HANDLE', 'GRAPHICS_PRESENT_OBJECT_ID']:
    assert token in handles, token
process = text('kernel/weavecore/src/process.rs')
for token in ['ServiceRole::Display', 'DISPLAY_PRESENT_HANDLE', 'GRAPHICS_PRESENT_OBJECT_ID', '[KERN] K13.', 'intentional post-userspace halt']:
    assert token in process, token

displayd = text('userspace/displayd/displayd.S')
for token in ['TW_SYS_GPU_PRESENT', 'TW_DISPLAY_PRESENT_HANDLE', 'display/compositor', 'capability-mediated buffered present verified']:
    assert token in displayd, token

runner = text('tools/run-k13c-qemu-gpu.sh')
for token in ['K13.C buffered compositor presentation QEMU test', 'virtio-gpu-pci', '-vga std', 'iommu_platform=off']:
    assert token in runner, token
checker = text('tools/check-k13c-serial-log.sh')
for token in ['[PRES]', '[DMG ]', '[PFEN]', '[GCOMP]', '[UPRS]', 'K13.C presentation/runtime qualification PASSED']:
    assert token in checker, token

print('Titanweave K13.C buffered compositor-presentation source checks passed.')
