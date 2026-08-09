#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
c=text('kernel/weavecore/src/native_gpu_c18.rs')
m=text('kernel/weavecore/src/main.rs')
for x in ['K14C18_ABI_VERSION: u32 = 1','AMD_DISCOVERY_TMR_SIZE: u32 = 10 * 1024','AMD_DISCOVERY_TMR_OFFSET: u64 = 64 * 1024','verify_snapshot_checksums','byte_sum16','checksum_self_test','RADEON_C18_LIVE_TMR_READ_ALLOWED: bool = false','RADEON_C18_LIVE_VRAM_READ_ALLOWED: bool = false','RADEON_C18_MMIO_WRITE_ALLOWED: bool = false','RADEON_C18_FIRMWARE_UPLOAD_ALLOWED: bool = false','RADEON_C18_COMMAND_SUBMIT_ALLOWED: bool = false','RADEON_C18_BUS_MASTER_ALLOWED: bool = false','[C18CK] discovery checksum verifier:','[C18PG] snapshot acquisition policy:','[C18HW] AMD discovery snapshot:','[C18RD] K14.C18 snapshot-verification gate ready:']:
    assert x in c,x
for x in ['mod native_gpu_c18;','native_gpu_c18::initialize()','[C18OK] K14.C18 AMD discovery snapshot-verification gate:']: assert x in m,x
assert 'SYS_NATIVE_GPU_C18_QUERY: u64 = 29' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C18_QUERY, 29' in text('userspace/include/twabi.inc')
s=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C18_QUERY' in s and 'native_gpu_c18::packed_status()' in s
d=text('userspace/displayd/displayd.S')
for x in ['K14.C18 AMD discovery snapshot-verification gate online','K14.C18 no physical Radeon in QEMU; bounded live discovery snapshot acquisition remains safely deferred']: assert x in d,x
p=text('kernel/weavecore/src/process.rs')
for x in ['[KERN] K14.C18 alive:','[QUAL] K14.C18 snapshot-verification runtime reached intentional post-userspace halt']: assert x in p,x
assert (root/'K14C18_IMPLEMENTATION.md').is_file()
assert (root/'K14C18_TESTER_GUIDE.md').is_file()
print('Titanweave K14.C18 AMD discovery snapshot-verification source checks passed.')
