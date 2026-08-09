#!/usr/bin/env python3
from pathlib import Path
import tomllib
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
with (root/'Cargo.toml').open('rb') as f: cargo=tomllib.load(f)
assert cargo['workspace']['package']['version']=='0.14.0'
main=text('kernel/weavecore/src/main.rs')
for token in ['mod translated_dma;', 'translated_dma::initialize_qualification(', '[IOMF] K14.B translated DMA qualification ready:']:
    assert token in main, token
hw=text('kernel/weavecore/src/translated_dma.rs')
for token in [
    'K14.B hardware-translated DMA qualification', 'QEMU_EDU_VENDOR', 'QEMU_EDU_DEVICE',
    'VTD_REG_RTADDR', 'VTD_GCMD_TE', 'VTD_CCMD_ICC', 'VTD_IOTLB_IVT',
    'TablePages', 'install_root_context', 'map_4k', 'unmap_4k', 'enable_translation',
    'disable_translation', 'invalidate_context_global', 'invalidate_iotlb_global',
    'forgebus::claim_pci_function', 'forgebus::establish_dma_domain',
    'pci::enable_bus_master(edu)', 'pci::disable_bus_master(edu)',
    '[IOMH]', '[IOVA]', '[DMAT]', '[IOPF]', '[INVL]', '[REVK]', '[IOMR]'
]: assert token in hw, token
# The endpoint must not be allowed to bus-master until after the VT-d root is installed and TE is enabled.
assert hw.index('vtd.set_root_table(tables.root)?;') < hw.index('vtd.enable_translation()?;') < hw.index('pci::enable_bus_master(edu);')
# Revoke ordering must disable bus mastering before tearing down the context / translation engine.
assert hw.index('pci::disable_bus_master(edu);', hw.index('[IOPF]')) < hw.index('tables.clear_context(requester);') < hw.index('vtd.disable_translation()?;')
# Hardware-translation qualification must be the only source of K14.B readiness promotion.
native=text('kernel/weavecore/src/native_gpu.rs')
for token in ['translated_dma::hardware_translation_qualified()', 'native_domain_bound = false', 'hardware_translation={}', 'device_domain_bound={}']:
    assert token in native, token
# K14.B still refuses native bus mastering until a native per-device domain exists.
assert '&& native_domain_bound' in native
backends=text('kernel/weavecore/src/k11_backends.rs')
for token in ['intel_primary_register_base', 'amd_primary_register_base']:
    assert token in backends, token
runner=text('tools/run-k14b-qemu-iommu.sh')
for token in ['K14.B hardware-translated DMA QEMU test', '-device edu,id=twk14iommutest', 'caching-mode=on', 'drive=k14bnvme', 'checker_status=0', 'check-k14b-serial-log.sh']:
    assert token in runner, token
checker=text('tools/check-k14b-serial-log.sh')
for token in ['[DMAT] EDU translated DMA round-trip verified:', '[IOPF] unmapped DMA denied:', 'destination_unchanged=true', 'K14.B translated-DMA runtime qualification PASSED']:
    assert token in checker, token
displayd=text('userspace/displayd/displayd.S')
for token in ['K14.B hardware-translated DMA engine qualified', 'shr rcx, 33', 'K14.B native GPU activation deferred']:
    assert token in displayd, token
process=text('kernel/weavecore/src/process.rs')
for token in ['[KERN] K14.B alive:', '[QUAL] K14.B translated-DMA runtime reached intentional post-userspace halt']:
    assert token in process, token
for rel in ['K14B_IMPLEMENTATION.md','K14B_TESTER_GUIDE.md','docs/architecture/K14.md']:
    assert (root/rel).is_file(), rel
print('Titanweave K14.B hardware-translated DMA source checks passed.')
