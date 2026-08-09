#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()

files=[
 'kernel/weavecore/src/radeon_mmio.rs',
 'kernel/weavecore/src/radeon_resources.rs',
 'kernel/weavecore/src/radeon_driver.rs',
 'kernel/weavecore/src/native_gpu_c27.rs',
]
for rel in files:
    t=text(rel)
    for forbidden in ['todo!(', 'unimplemented!(', 'TODO:', 'placeholder implementation', 'stub implementation']:
        assert forbidden not in t, f'{rel}: forbidden stub marker {forbidden}'

mm=text('kernel/weavecore/src/radeon_mmio.rs')
for x in [
 'RADEON_MMIO_SERVICE_ABI_VERSION: u32 = 1',
 'ReviewedRegister', 'ScratchReg0', 'ScratchReg1', 'AccessClass',
 'authorize_generic_write', 'generic Radeon MMIO writes are forbidden',
 'read_reviewed', 'map_kernel_mmio_readonly', 'read_volatile',
 'RADEON_MMIO_GENERIC_WRITE_ALLOWED: bool = false',
 'RADEON_MMIO_CALLER_ADDRESS_ALLOWED: bool = false',
 'RADEON_MMIO_CALLER_VALUE_ALLOWED: bool = false',
]: assert x in mm,x
assert 'write_volatile' not in mm

rs=text('kernel/weavecore/src/radeon_resources.rs')
for x in [
 'RADEON_RESOURCE_ABI_VERSION:u32=1', 'forgebus::device_id_for_pci',
 'forgebus::driver_id_for_device', 'forgebus::device_state',
 'bar0_vram_aperture_bytes', 'bar5_mmio_base', 'iommu_hardware_translated',
 'persistent_device_domain', 'topology_verified',
]: assert x in rs,x

rd=text('kernel/weavecore/src/radeon_driver.rs')
for x in [
 'RADEON_DRIVER_CORE_ABI_VERSION:u32=1', 'CorePhase', 'CoreMachine',
 'fn irq_handler', 'interrupt_router_self_test', 'record_dispatch',
 'register_handler', 'route.masked', 'machine_self_test', '.coordinate_reset()',
 'RADEON_C27_HARDWARE_IRQ_ENABLE_ALLOWED:bool=false',
 'RADEON_C27_FIRMWARE_UPLOAD_ALLOWED:bool=false',
 'RADEON_C27_DMA_ENABLE_ALLOWED:bool=false',
 'RADEON_C27_COMMAND_SUBMIT_ALLOWED:bool=false',
]: assert x in rd,x

c=text('kernel/weavecore/src/native_gpu_c27.rs')
for x in [
 'K14C27_ABI_VERSION:u32=1', 'RADEON_C27_PLACEHOLDER_SUBSYSTEMS:u8=0',
 'radeon_resources::initialize()', 'radeon_mmio::initialize(allocator,kernel_cr3)',
 'radeon_driver::initialize(resources,mmio)',
 '[C27DV]', '[C27RS]', '[C27MM]', '[C27IR]', '[C27ER]', '[C27PG]', '[C27RD]',
 'firmware=false DMA=false bus_master=false submit=false',
 'no_placeholders=true',
]: assert x in c,x

fb=text('kernel/weavecore/src/forgebus.rs')
for x in ['device_id_for_pci','driver_id_for_device','device_state']:
    assert x in fb,x

m=text('kernel/weavecore/src/main.rs')
for x in ['mod radeon_mmio;','mod radeon_resources;','mod radeon_driver;','mod native_gpu_c27;',
          'native_gpu_c27::initialize(&mut allocator','[C27OK] K14.C27 complete Radeon driver core:']:
    assert x in m,x
s=text('kernel/weavecore/src/syscalls.rs')
assert 'SYS_NATIVE_GPU_C27_QUERY' in s and 'native_gpu_c27::packed_status()' in s
assert 'SYS_NATIVE_GPU_C27_QUERY: u64 = 38' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C27_QUERY, 38' in text('userspace/include/twabi.inc')
d=text('userspace/displayd/displayd.S')
for x in ['K14.C27 complete Radeon driver core online','operational driver-core software paths qualified',
          'TW_SYS_NATIVE_GPU_C27_QUERY','test eax, 0x2000']:
    assert x in d,x
p=text('kernel/weavecore/src/process.rs')
for x in ['[K14FOUND]','[KERN] K14.C27 alive:','[QUAL] K14.C27 complete-radeon-driver-core runtime reached intentional post-userspace halt']:
    assert x in p,x
print('Titanweave K14.C27 operational Radeon driver-core source checks passed.')
