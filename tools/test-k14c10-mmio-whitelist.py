#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
main=text('kernel/weavecore/src/main.rs')
for t in ['mod native_gpu_c10;','native_gpu_c10::initialize()','[C10OK] K14.C10 guarded MMIO-read engine:']: assert t in main,t
c=text('kernel/weavecore/src/native_gpu_c10.rs')
for t in ['K14C10_ABI_VERSION','MmioReadDescriptor','ProfileMmioWhitelist','VERIFIED_MMIO_WHITELISTS','NAVI21_MMIO_READS','NAVI48_MMIO_READS','descriptor_valid','read_descriptor','RADEON_C10_MAX_MMIO_READS','[C10WL]','[C10RD]','[C10HW]','[C10NF]']: assert t in c,t
for t in ['RADEON_C10_MMIO_WRITES_ALLOWED: bool = false','RADEON_C10_FIRMWARE_UPLOAD_ALLOWED: bool = false','RADEON_C10_COMMAND_SUBMIT_ALLOWED: bool = false','RADEON_C10_BUS_MASTER_ALLOWED: bool = false']: assert t in c,t
assert 'SYS_NATIVE_GPU_C10_QUERY: u64 = 21' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C10_QUERY, 21' in text('userspace/include/twabi.inc')
sys=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C10_QUERY' in sys and 'native_gpu_c10::packed_status()' in sys
d=text('userspace/displayd/displayd.S')
for t in ['K14.C10 per-IP MMIO whitelist and guarded live-read engine online','no physical Radeon in QEMU','MMIO reads remain fenced']: assert t in d,t
p=text('kernel/weavecore/src/process.rs')
for t in ['[KERN] K14.C10 alive:','[QUAL] K14.C10 guarded Radeon MMIO-read runtime reached intentional post-userspace halt']: assert t in p,t
for f in ['K14C10_IMPLEMENTATION.md','K14C10_TESTER_GUIDE.md']: assert (root/f).exists(),f
print('Titanweave K14.C10 per-IP MMIO whitelist/live-read source checks passed.')
