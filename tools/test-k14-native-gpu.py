#!/usr/bin/env python3
from pathlib import Path
import re, tomllib
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
with (root/'Cargo.toml').open('rb') as f: cargo=tomllib.load(f)
assert cargo['workspace']['package']['version']=='0.14.0'
boot=text('libraries/boot-protocol/src/lib.rs')
for token in ['BOOT_PROTOCOL_VERSION: u32 = 14','TITANV14','FramebufferInfo']:
    assert token in boot, token
native=text('kernel/weavecore/src/native_gpu.rs')
for token in [
    'K14.A native GPU prerequisite', 'NativeGpuBackend', 'NativeDriverPhase',
    'NativeIommuReadiness', 'HardwareTranslated', 'discover_probe_only',
    'authorize_bus_mastering', 'native GPU DMA requires hardware-translated IOMMU mappings',
    'probe_only=true', '[NDRV]', '[NGPU]', '[IOMQ]', '[NFAL]'
]: assert token in native, token
# Discovery itself must remain read-only and unowned.
start=native.index('pub fn discover_probe_only')
end=native.index('pub fn current_iommu_readiness', start)
probe=native[start:end]
for forbidden in ['write_u32(', 'enable_bus_master(', 'enable_memory_decode(', 'claim_pci_function(', 'map_kernel_mmio(']:
    assert forbidden not in probe, forbidden
assert 'pci::read_u16' in probe and 'memory_bar_count(function)' in probe
bar_helper=native[native.index('fn memory_bar_count'):native.index('pub fn discover_probe_only')]
assert 'pci::read_u32' in bar_helper
# K14.B may promote readiness only through the live translated-DMA qualification.
iommu=native[native.index('pub fn current_iommu_readiness'):native.index('pub fn authorize_bus_mastering')]
assert 'PolicyOnly' in iommu
assert 'translated_dma::hardware_translation_qualified()' in iommu
assert 'return NativeIommuReadiness::HardwareTranslated' in iommu
main=text('kernel/weavecore/src/main.rs')
for token in ['mod native_gpu;', 'WeaveCore K14', 'native_gpu::initialize_foundation()', '[NATF] K14.A native GPU prerequisite foundation ready:']:
    assert token in main, token
abi=text('kernel/weavecore/src/abi.rs'); assert 'SYS_NATIVE_GPU_QUERY: u64 = 11' in abi
inc=text('userspace/include/twabi.inc'); assert 'TW_SYS_NATIVE_GPU_QUERY, 11' in inc
syscalls=text('kernel/weavecore/src/syscalls.rs')
for token in ['SYS_NATIVE_GPU_QUERY', 'native_gpu::packed_status()', '[SHELL] native gpu status:']:
    assert token in syscalls, token
displayd=text('userspace/displayd/displayd.S')
for token in ['TW_SYS_NATIVE_GPU_QUERY', 'K14.A native GPU candidate visible', 'K14.A native GPU activation deferred']:
    assert token in displayd, token
process=text('kernel/weavecore/src/process.rs')
for token in ['[KERN] K14.A alive:', '[QUAL] K14.A native-GPU foundation runtime reached intentional post-userspace halt']:
    assert token in process, token
runner=text('tools/run-k14-qemu-native-gpu.sh')
for token in ['K14.A native-GPU prerequisite QEMU test', 'virtio-gpu-pci', 'iommu_platform=off', '-vga std']:
    assert token in runner, token
checker=text('tools/check-k14-serial-log.sh')
for token in ['[NDRV]', '[NGPU]', '[IOMQ]', '[NATF]', 'K14.A native-GPU foundation/runtime qualification PASSED']:
    assert token in checker, token
assert (root/'docs/architecture/K14.md').is_file()
assert (root/'K14_STATUS.md').is_file()
assert (root/'K14A_IMPLEMENTATION.md').is_file()
assert (root/'K14_TESTER_GUIDE.md').is_file()
print('Titanweave K14.A native-GPU prerequisite source checks passed.')
