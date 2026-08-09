#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
main=text('kernel/weavecore/src/main.rs')
for x in ['mod native_gpu_c13;','native_gpu_c13::initialize()','[C13OK] K14.C13 physical Radeon read-proof engine:']: assert x in main,x
c=text('kernel/weavecore/src/native_gpu_c13.rs')
for x in ['K14C13_ABI_VERSION','RADEON_C13_REQUIRED_READS','fingerprint(','[C13PV]','[C13SN]','[C13HW]','[C13RD]','navi48_discovery_pending']: assert x in c,x
for x in ['RADEON_C13_MMIO_WRITES_ALLOWED: bool = false','RADEON_C13_FIRMWARE_UPLOAD_ALLOWED: bool = false','RADEON_C13_COMMAND_SUBMIT_ALLOWED: bool = false','RADEON_C13_BUS_MASTER_ALLOWED: bool = false']: assert x in c,x
assert 'SYS_NATIVE_GPU_C13_QUERY: u64 = 24' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C13_QUERY, 24' in text('userspace/include/twabi.inc')
s=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C13_QUERY' in s and 'native_gpu_c13::packed_status()' in s
d=text('userspace/displayd/displayd.S')
for x in ['K14.C13 physical Radeon read-proof qualification gate online','no physical Radeon in QEMU','physical Radeon read proof complete']: assert x in d,x
p=text('kernel/weavecore/src/process.rs')
for x in ['[KERN] K14.C13 alive:','[QUAL] K14.C13 physical-read-proof runtime reached intentional post-userspace halt']: assert x in p,x
for f in ['K14C13_IMPLEMENTATION.md','K14C13_TESTER_GUIDE.md']: assert (root/f).exists(),f
print('Titanweave K14.C13 physical Radeon read-proof source checks passed.')
