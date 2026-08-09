#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]

def text(path: str) -> str:
    return (root / path).read_text()

resilience = text('kernel/weavecore/src/gpu_resilience.rs')
for token in [
    'K13.D GPU resilience', 'GpuHealthManager', 'AdapterHealthState',
    'GPU_STALL_RECOVERY_THRESHOLD', 'begin_recovery', 'complete_rebind',
    'surprise_remove', 'ScanoutTopology', 'MAX_MANAGED_SCANOUTS',
    'HotplugController', 'TransferRoute::PeerToPeer', 'run_self_test',
]:
    assert token in resilience, token

runtime = text('kernel/weavecore/src/gpu_runtime.rs')
for token in [
    'initialize_resilience_qualification', '[RSLN]', '[HOTG]', '[MOUT]', '[MGP2]',
    '[SOAK]', '[DLOS]', '[REBD]', 'recover_from_displayd', '[URCV]',
    'resilience_ready: true', 'SOAK_FRAMES: u32 = 64',
]:
    assert token in runtime, token

virtio = text('kernel/weavecore/src/virtio_gpu.rs')
for token in [
    'presentation_suspended', 'suspend_presentation_for_recovery',
    'resume_presentation_after_recovery', 'VirtioGpuRecoveryReport',
    'presentation is suspended for recovery', 'Full PCI FLR/slot reset is intentionally not claimed',
]:
    assert token in virtio, token

# Recovery must reuse the already-owned K13.B transport. It must not discover,
# claim, enable bus mastering, or allocate a second unowned transport.
start = runtime.index('pub fn initialize_resilience_qualification')
end = runtime.index('pub fn recover_from_displayd', start)
block = runtime[start:end]
for forbidden in ['claim_pci_function', 'enable_bus_master', 'initialize_transport(', 'allocate_dma(']:
    assert forbidden not in block, forbidden
assert 'present_compositor_frame' in block
assert 'suspend_presentation_for_recovery' in block
assert 'resume_presentation_after_recovery' in block

main = text('kernel/weavecore/src/main.rs')
for token in [
    'mod gpu_resilience;', 'gpu_runtime::initialize_resilience_qualification()',
    '[GRDY] K13.D resilience/multi-GPU ready:',
]:
    assert token in main, token

abi = text('kernel/weavecore/src/abi.rs')
assert 'SYS_GPU_RECOVER: u64 = 10' in abi
inc = text('userspace/include/twabi.inc')
assert 'TW_SYS_GPU_RECOVER, 10' in inc

syscalls = text('kernel/weavecore/src/syscalls.rs')
for token in ['SYS_GPU_RECOVER', 'GRAPHICS_PRESENT_OBJECT_ID', 'gpu_runtime::recover_from_displayd', 'note_displayd_recovery_result', 'acknowledge_displayd_recovery_write']:
    assert token in syscalls, token

# Recovery is authorized by the existing DISPLAYD-only graphics capability,
# never by PID/name matching or direct userspace MMIO access.
recover_case = syscalls[syscalls.index('SYS_GPU_RECOVER =>'):]
assert 'current_lookup(a1 as Handle, RIGHT_WRITE)' in recover_case
assert 'GRAPHICS_PRESENT_OBJECT_ID' in recover_case
assert 'process name' not in recover_case

displayd = text('userspace/displayd/displayd.S')
for token in [
    'K13.D display/compositor service online; resilience enabled',
    'TW_SYS_GPU_RECOVER', 'TW_DISPLAY_PRESENT_HANDLE',
    'K13.D capability-mediated GPU recovery verified',
]:
    assert token in displayd, token

process = text('kernel/weavecore/src/process.rs')
for token in [
    '[KERN] K13.D alive:', '[QUAL] K13.D robustness runtime reached intentional post-userspace halt',
    'shell_completed: bool', 'displayd_recovery_result: i8', 'displayd_recovery_acknowledged: bool',
    'note_displayd_recovery_result', 'acknowledge_displayd_recovery_write',
    'shell and DISPLAYD recovery milestones complete',
]:
    assert token in process, token
assert 'finish_runtime_with_result(0);' not in process[process.index('if SERVICE_SPECS[index].role == ServiceRole::Shell'):process.index('if all_terminal(count)', process.index('if SERVICE_SPECS[index].role == ServiceRole::Shell'))]

runner = text('tools/run-k13d-qemu-gpu.sh')
for token in [
    'K13.D multi-GPU/resilience QEMU test', '-vga std',
    'twk13gpu0', 'twk13gpu1', 'iommu_platform=off',
]:
    assert token in runner, token
assert runner.count('virtio-gpu-pci') >= 2

checker = text('tools/check-k13d-serial-log.sh')
for token in [
    '[RSLN]', '[HOTG]', '[MOUT]', '[MGP2]', '[SOAK]', '[DLOS]', '[REBD]', '[GRDY]',
    '[URCV]', 'K13.D robustness/runtime qualification PASSED',
]:
    assert token in checker, token

print('Titanweave K13.D resilience/multi-GPU source checks passed.')
