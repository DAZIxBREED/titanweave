#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
c=text('kernel/weavecore/src/native_gpu_c19.rs')
m=text('kernel/weavecore/src/main.rs')
for x in [
 'K14C19_ABI_VERSION: u32 = 1',
 'RADEON_C19_MMIO_BAR_INDEX: u8 = 5',
 'RADEON_C19_MAX_LOCATOR_READS: u8 = 4',
 'RADEON_C19_LOCATOR_READS_ALLOWED: bool = true',
 'RADEON_C19_LIVE_TMR_PAYLOAD_READ_ALLOWED: bool = false',
 'RADEON_C19_LIVE_VRAM_READ_ALLOWED: bool = false',
 'RADEON_C19_MMIO_WRITE_ALLOWED: bool = false',
 'RADEON_C19_FIRMWARE_UPLOAD_ALLOWED: bool = false',
 'RADEON_C19_COMMAND_SUBMIT_ALLOWED: bool = false',
 'RADEON_C19_BUS_MASTER_ALLOWED: bool = false',
 'derive_locator',
 'read_ro_reg',
 '[C19PG] physical discovery-locator policy:',
 '[C19HW] physical AMD discovery locator:',
 '[C19RD] K14.C19 physical-locator gate ready:',
]:
    assert x in c,x
for x in [
 'mod native_gpu_c19;',
 'native_gpu_c19::initialize(&mut allocator, boot_info.bootstrap.page_table_root)',
 '[C19OK] K14.C19 physical AMD discovery-locator gate:',
]:
    assert x in m,x
assert 'SYS_NATIVE_GPU_C19_QUERY: u64 = 30' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C19_QUERY, 30' in text('userspace/include/twabi.inc')
s=text('kernel/weavecore/src/syscalls.rs')
assert 'SYS_NATIVE_GPU_C19_QUERY' in s and 'native_gpu_c19::packed_status()' in s
d=text('userspace/displayd/displayd.S')
for x in [
 'K14.C19 physical AMD discovery-locator gate online',
 'K14.C19 no physical Radeon in QEMU; physical discovery-locator reads remain safely deferred'
]:
    assert x in d,x
p=text('kernel/weavecore/src/process.rs')
for x in [
 '[KERN] K14.C19 alive:',
 '[QUAL] K14.C19 physical-locator runtime reached intentional post-userspace halt'
]:
    assert x in p,x
assert (root/'K14C19_IMPLEMENTATION.md').is_file()
assert (root/'K14C19_TESTER_GUIDE.md').is_file()
print('Titanweave K14.C19 physical AMD discovery-locator source checks passed.')
