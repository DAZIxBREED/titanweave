#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
main=text('kernel/weavecore/src/main.rs')
for x in ['mod native_gpu_c14;','native_gpu_c14::initialize()','[C14OK] K14.C14 controlled write-promotion readiness gate:']: assert x in main,x
c=text('kernel/weavecore/src/native_gpu_c14.rs')
for x in ['K14C14_ABI_VERSION','RADEON_C14_WRITE_PROMOTION_ALLOWED','[C14PG]','[C14CK]','[C14HW]','[C14RD]','write_prerequisites_complete','write_promotion_enabled']: assert x in c,x
for x in [
 'RADEON_C14_MMIO_WRITES_ALLOWED: bool = false',
 'RADEON_C14_FIRMWARE_UPLOAD_ALLOWED: bool = false',
 'RADEON_C14_COMMAND_SUBMIT_ALLOWED: bool = false',
 'RADEON_C14_BUS_MASTER_ALLOWED: bool = false',
 'RADEON_C14_WRITE_PROMOTION_ALLOWED: bool = false'
]: assert x in c,x
assert 'SYS_NATIVE_GPU_C14_QUERY: u64 = 25' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C14_QUERY, 25' in text('userspace/include/twabi.inc')
s=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C14_QUERY' in s and 'native_gpu_c14::packed_status()' in s
d=text('userspace/displayd/displayd.S')
for x in ['K14.C14 controlled Radeon write-promotion readiness gate online','no physical Radeon in QEMU','write-side prerequisites proven']: assert x in d,x
p=text('kernel/weavecore/src/process.rs')
for x in ['[KERN] K14.C14 alive:','[QUAL] K14.C14 write-promotion-readiness runtime reached intentional post-userspace halt']: assert x in p,x
for f in ['K14C14_IMPLEMENTATION.md','K14C14_TESTER_GUIDE.md']: assert (root/f).exists(),f
print('Titanweave K14.C14 controlled write-promotion readiness source checks passed.')
