#!/usr/bin/env python3
from pathlib import Path
import tomllib
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
with (root/'Cargo.toml').open('rb') as f: cargo=tomllib.load(f)
assert cargo['workspace']['package']['version']=='0.14.0'
main=text('kernel/weavecore/src/main.rs')
for token in ['mod amd_gpu;', 'mod native_gpu_binding;', 'native_gpu_binding::initialize_binding_foundation()', '[NCF ] K14.C1 native binding foundation ready:']:
    assert token in main, token
amd=text('kernel/weavecore/src/amd_gpu.rs')
for token in ['K14.C1 AMD-first native GPU backend foundation', 'AMD_NATIVE_BACKEND_ABI_VERSION', 'is_amd_display', 'claim_foundation', 'forgebus::claim_pci_function', 'forgebus::establish_dma_domain', 'pci::disable_bus_master(function)', 'bus_master_enabled: false']:
    assert token in amd, token
# C1 may clear a preexisting bus-master bit but must never enable bus mastering or touch vendor MMIO.
assert 'pci::enable_bus_master' not in amd
assert 'map_kernel_mmio' not in amd
binding=text('kernel/weavecore/src/native_gpu_binding.rs')
for token in ['NATIVE_BINDING_ABI_VERSION', 'select_native_candidate', 'GpuMemoryManager', 'MemoryDomain::Vram', 'MemoryDomain::Gtt', 'persistent_device_domain = false', '[AMDB]', '[NVRM]', '[NSEL]', '[NBND]', '[NDOM]', '[NCF ]']:
    assert token in binding, token
assert binding.index('report.persistent_device_domain = false;') < binding.index('report.bus_master_enabled = false;')
abi=text('kernel/weavecore/src/abi.rs'); assert 'SYS_NATIVE_GPU_BINDING_QUERY: u64 = 12' in abi
inc=text('userspace/include/twabi.inc'); assert 'TW_SYS_NATIVE_GPU_BINDING_QUERY, 12' in inc
syscalls=text('kernel/weavecore/src/syscalls.rs')
for token in ['SYS_NATIVE_GPU_BINDING_QUERY', 'native_gpu_binding::packed_status()', 'native gpu binding status']:
    assert token in syscalls, token
displayd=text('userspace/displayd/displayd.S')
for token in ['TW_SYS_NATIVE_GPU_BINDING_QUERY', 'K14.C1 native backend ownership foundation online', 'K14.C1 waiting for bare-metal AMD/Intel/NVIDIA adapter']:
    assert token in displayd, token
process=text('kernel/weavecore/src/process.rs')
for token in ['[KERN] K14.C1 alive:', '[QUAL] K14.C1 native binding runtime reached intentional post-userspace halt']:
    assert token in process, token
runner=text('tools/run-k14c1-qemu-native-binding.sh')
for token in ['K14.C1 native GPU ownership QEMU test', 'check-k14c1-serial-log.sh', '-device edu,id=twk14c1iommutest', 'iommu_platform=off']:
    assert token in runner, token
checker=text('tools/check-k14c1-serial-log.sh')
for token in ['[AMDB]', '[NVRM]', '[NSEL]', '[NBND]', '[NDOM]', '[NCF ]', 'K14.C1 native binding/runtime qualification PASSED']:
    assert token in checker, token
for rel in ['K14C1_IMPLEMENTATION.md','K14C1_TESTER_GUIDE.md','K14_STATUS.md','docs/architecture/K14.md']:
    assert (root/rel).is_file(), rel
print('Titanweave K14.C1 native GPU ownership foundation source checks passed.')
