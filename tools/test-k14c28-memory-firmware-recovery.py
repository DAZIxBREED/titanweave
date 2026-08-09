#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
files=['kernel/weavecore/src/radeon_memory.rs','kernel/weavecore/src/radeon_firmware.rs','kernel/weavecore/src/radeon_recovery.rs','kernel/weavecore/src/native_gpu_c28.rs']
for rel in files:
    t=text(rel)
    for forbidden in ['todo!(', 'unimplemented!(', 'TODO:', 'placeholder implementation', 'stub implementation']:
        assert forbidden not in t, f'{rel}: forbidden stub marker {forbidden}'

pg=text('kernel/weavecore/src/paging.rs')
for x in ['KERNEL_DMA_BASE','KERNEL_DMA_LIMIT','map_kernel_dma(', 'map_kernel_dma_page(', 'unmap_kernel_dma(',
          'physical_address | PAGE_PRESENT | PAGE_WRITABLE | PAGE_NO_EXECUTE']:
    assert x in pg,x
mem=text('kernel/weavecore/src/radeon_memory.rs')
for x in ['RadeonMemoryManager','reserve_vram','allocate_gtt','map_kernel_dma','unmap_kernel_dma','deallocate_contiguous',
          'real_gtt_self_test','write_volatile','read_volatile','IovaAllocator::new','RADEON_GPU_VA_BASE','gpu_virtual','gpu_va.allocate','gpu_va.free','RADEON_C28_GPU_PAGE_TABLES_INSTALLED:bool=false',
          'RADEON_C28_DEVICE_DMA_ENABLED:bool=false','gpu_va_allocator_ready','gpu_va_reservation_verified']:
    assert x in mem,x
fw=text('kernel/weavecore/src/radeon_firmware.rs')
for x in ['CommonFirmwareHeader','parse_common_header','crc32_ieee','0xedb8_8320','0xcbf4_3926','vfs::file_exists',
          'vfs::with_file','sha256::digest','radeon_memory::allocate_gtt','radeon_memory::pin','GFXRLC.BIN','GFXPFP.BIN',
          'RADEON_C28_FIRMWARE_UPLOAD_TO_GPU:bool=false']:
    assert x in fw,x
rec=text('kernel/weavecore/src/radeon_recovery.rs')
for x in ['DriverWatchdog','WatchdogAction::Ping','WatchdogAction::Restart','WatchdogAction::Quarantine',
          'quiesce_for_recovery','record_recovery_fault','coordinate_recovery','resume_after_recovery',
          'mask_recovery_interrupt_route','activate_recovery_interrupt_route','interrupt_route_active','radeon_memory::free','RADEON_C28_PHYSICAL_ASIC_RESET_PERFORMED:bool=false']:
    assert x in rec,x
rd=text('kernel/weavecore/src/radeon_driver.rs')
for x in ['pub fn quiesce_for_recovery','pub fn record_recovery_fault','pub fn coordinate_recovery','pub fn resume_after_recovery']:
    assert x in rd,x
c=text('kernel/weavecore/src/native_gpu_c28.rs')
for x in ['K14C28_ABI_VERSION:u32=1','RADEON_C28_PLACEHOLDER_SUBSYSTEMS:u8=0','radeon_memory::initialize',
          'radeon_firmware::initialize','radeon_recovery::initialize','[C28ME]','[C28FW]','[C28RC]','[C28PG]','[C28RD]',
          'GPU_VA_reservations=true GPU_page_tables=false DMA=false bus_master=false submit=false recovery_IRQ_route=active_if_physical',
          'no_placeholders=true','gpu_va_allocator_ready','recovery_interrupt_route_active']:
    assert x in c,x
main=text('kernel/weavecore/src/main.rs')
for x in ['mod radeon_memory;','mod radeon_firmware;','mod radeon_recovery;','mod native_gpu_c28;',
          'vfs::mount_boot_volume(boot_info)','native_gpu_c28::initialize(&mut allocator','[C28OK] K14.C28 Radeon memory+firmware+recovery:']:
    assert x in main,x
assert main.index('vfs::mount_boot_volume(boot_info)') < main.index('native_gpu_c28::initialize(&mut allocator')
assert 'SYS_NATIVE_GPU_C28_QUERY: u64 = 39' in text('kernel/weavecore/src/abi.rs')
s=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C28_QUERY' in s and 'native_gpu_c28::packed_status()' in s
assert 'TW_SYS_NATIVE_GPU_C28_QUERY, 39' in text('userspace/include/twabi.inc')
d=text('userspace/displayd/displayd.S')
for x in ['K14.C28 Radeon memory, firmware staging, and recovery subsystem online','TW_SYS_NATIVE_GPU_C28_QUERY',
          'real GTT allocation/mapping/reclaim plus firmware parser/CRC','test eax, 0x2000']:
    assert x in d,x
p=text('kernel/weavecore/src/process.rs')
for x in ['[KERN] K14.C28 alive:','[QUAL] K14.C28 memory-firmware-recovery runtime reached intentional post-userspace halt','[K14FOUND]']:
    assert x in p,x
assert 'const FILE_SCRATCH_BYTES: usize = 1024 * 1024;' in text('kernel/weavecore/src/vfs.rs')
print('Titanweave K14.C28 operational memory+firmware+recovery source checks passed.')
