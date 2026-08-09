#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
def text(path: str) -> str:
    return (root / path).read_text()

virtio = text('kernel/weavecore/src/virtio_gpu.rs')
for token in [
    'K13.B VirtIO-GPU modern PCI transport',
    'VIRTIO_PCI_CAP_COMMON_CFG', 'VIRTIO_PCI_CAP_NOTIFY_CFG',
    'VIRTIO_PCI_CAP_ISR_CFG', 'VIRTIO_PCI_CAP_DEVICE_CFG',
    'VIRTIO_F_VERSION_1_HIGH_BIT', 'discover_capabilities', 'negotiate_features',
    'setup_queue', 'CONTROL_QUEUE_INDEX', 'CURSOR_QUEUE_INDEX',
    'VIRTIO_GPU_CMD_GET_DISPLAY_INFO', 'VIRTIO_GPU_CMD_RESOURCE_CREATE_2D',
    'VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING', 'VIRTIO_GPU_CMD_SET_SCANOUT',
    'VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D', 'VIRTIO_GPU_CMD_RESOURCE_FLUSH',
    'forgebus::claim_pci_function', 'forgebus::establish_dma_domain',
    'forgebus::allocate_dma', 'pci::enable_bus_master', 'pci::disable_bus_master',
    'transport_ready: true', 'hardware-IOMMU backends are not yet a complete',
    'paging::map_kernel_mmio', '[MMIO] VirtIO cap type=',
]:
    assert token in virtio, token

# Safety ordering: ownership and bounded DMA bookkeeping must precede bus mastering.
claim = virtio.index('forgebus::claim_pci_function')
domain = virtio.index('forgebus::establish_dma_domain')
memdecode = virtio.index('pci::enable_memory_decode')
busmaster = virtio.index('pci::enable_bus_master')
assert claim < domain < memdecode < busmaster

# K13.B deliberately must not advertise ACCESS_PLATFORM until real hardware
# translation/page-table programming exists.
assert 'let negotiated_high = VIRTIO_F_VERSION_1_HIGH_BIT;' in virtio
assert 'VIRTIO_F_VERSION_1_HIGH_BIT | VIRTIO_F_ACCESS_PLATFORM_HIGH_BIT' not in virtio

pci = text('kernel/weavecore/src/pci.rs')
for token in ['read_u8', 'read_u16', 'memory_bar_base', 'enable_memory_decode', 'enable_bus_master', 'disable_bus_master']:
    assert token in pci, token

bus = text('kernel/weavecore/src/forgebus.rs')
for token in ['claim_pci_function', 'establish_dma_domain', 'allocate_dma', 'release_dma', 'mark_device_online', 'revoke_device_dma']:
    assert token in bus, token

runtime = text('kernel/weavecore/src/gpu_runtime.rs')
for token in [
    'initialize_transport', '[VPCI]', '[VQ  ]', '[VDMA]', '[SCAN]',
    'transport passed transport_ready={}', 'backend=virtio-gpu-modern',
    'hardware_translation=deferred',
]:
    assert token in runtime, token

main = text('kernel/weavecore/src/main.rs')
assert 'gpu_runtime::initialize_transport(' in main
assert 'boot_info.bootstrap.page_table_root' in main
assert '[GPUT] K13.B VirtIO-GPU transport ready:' in main

displayd = text('userspace/displayd/displayd.S')
for token in ['display/compositor service online', 'VirtIO-GPU command transport online', '0x80000000']:
    assert token in displayd, token

runner = text('tools/run-k13b-qemu-gpu.sh')
for token in ['virtio-gpu-pci', 'iommu_platform=off', '-vga std', 'K13.B VirtIO-GPU transport QEMU test', 'K13_IOMMU']:
    assert token in runner, token

checker = text('tools/check-k13b-serial-log.sh')
for token in ['[VPCI]', '[VQ  ]', '[VDMA]', '[SCAN]', 'transport_ready=true', 'K13.B transport/runtime qualification PASSED']:
    assert token in checker, token


paging = text('kernel/weavecore/src/paging.rs')
for token in [
    'KERNEL_MMIO_BASE', 'KERNEL_MMIO_LIMIT', 'map_kernel_mmio',
    'PAGE_CACHE_DISABLE', 'PAGE_WRITE_THROUGH', 'PAGE_NO_EXECUTE',
    'ensure_kernel_table', 'invlpg',
]:
    assert token in paging, token
assert 'VirtIO capability lies outside bootstrap identity map' not in virtio
print('Titanweave K13.B VirtIO-GPU transport source checks passed.')
