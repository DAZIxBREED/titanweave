#!/usr/bin/env python3
from pathlib import Path
import tomllib

root = Path(__file__).resolve().parents[1]
def text(path: str) -> str:
    return (root / path).read_text()

with (root / 'Cargo.toml').open('rb') as f:
    cargo = tomllib.load(f)
version = tuple(int(part) for part in cargo['workspace']['package']['version'].split('.'))
assert version >= (0, 13, 0)

boot = text('libraries/boot-protocol/src/lib.rs')
match = __import__('re').search(r'BOOT_PROTOCOL_VERSION: u32 = (\d+)', boot)
assert match and int(match.group(1)) >= 13
assert 'FramebufferInfo' in boot

checks = {
    'kernel/weavecore/src/gpu_topology.rs': [
        'PCI_CLASS_DISPLAY', 'VENDOR_AMD', 'VENDOR_INTEL', 'VENDOR_NVIDIA',
        'VENDOR_VIRTIO', 'discover',
    ],
    'kernel/weavecore/src/gpu_memory.rs': [
        'GpuMemoryManager', 'MemoryDomain', 'Vram', 'Gtt', 'migrate', 'pin',
    ],
    'kernel/weavecore/src/gpu_queue.rs': [
        'CommandQueue', 'GPU_QUEUE_DEPTH', 'CommandPacket', 'CopyBetweenAdapters',
    ],
    'kernel/weavecore/src/gpu_fence.rs': ['TimelineFence', 'issue', 'complete'],
    'kernel/weavecore/src/gpu_modeset.rs': ['AtomicModeRequest', 'DisplayMode', 'enable_vrr', 'enable_hdr'],
    'kernel/weavecore/src/gpu_multigpu.rs': ['TransferRoute', 'PeerToPeer', 'SharedSystemMemory', 'CpuStaging'],
    'kernel/weavecore/src/virtio_gpu.rs': ['K13.B VirtIO-GPU modern PCI transport', 'VIRTIO_GPU_MODERN_DEVICE', 'bus_master_enabled', 'pub fn probe'],
    'kernel/weavecore/src/gpu_runtime.rs': [
        'FORGEGRAPHICS_ACCEL_ABI_VERSION', 'transport_ready: false',
        'K13 topology', 'domain lifecycle self-test', 'bounded submission self-test',
        'timeline self-test', 'atomic modeset contract', 'transfer policy self-test',
    ],
    'kernel/weavecore/src/abi.rs': ['SYS_GPU_QUERY'],
    'kernel/weavecore/src/syscalls.rs': ['SYS_GPU_QUERY', 'gpu_runtime::packed_status'],
    'userspace/include/twabi.inc': ['TW_SYS_GPU_QUERY'],
    'userspace/displayd/displayd.S': ['display/compositor service online', 'TW_SYS_GPU_QUERY'],
    'tools/run-k13-qemu-gpu.sh': ['virtio-gpu-pci', '-vga std', 'K13_IOMMU'],
    'tools/check-k13-serial-log.sh': ['K13.A GPU-foundation runtime qualification PASSED'],
}
for path, tokens in checks.items():
    source = text(path)
    for token in tokens:
        assert token in source, (path, token)

assert (root / 'docs/architecture/K13.md').is_file()
assert (root / 'K13_TESTER_GUIDE.md').is_file()

# K13.A regression invariant: discovery itself remains side-effect free even
# though this K13.B tree now contains a separate transport initializer.
virtio = text('kernel/weavecore/src/virtio_gpu.rs')
probe_block = virtio.split('pub fn probe()', 1)[1].split('fn find_function', 1)[0]
assert 'enable_bus_master' not in probe_block
assert 'enable_memory_decode' not in probe_block
assert 'write32' not in probe_block

print('Titanweave K13.A GPU acceleration-foundation regression checks passed.')
