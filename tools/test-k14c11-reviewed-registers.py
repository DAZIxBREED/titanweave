#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
main=text('kernel/weavecore/src/main.rs')
for t in ['mod native_gpu_c11;','native_gpu_c11::initialize()','[C11OK] K14.C11 reviewed register/IP-base gate:']: assert t in main,t
c=text('kernel/weavecore/src/native_gpu_c11.rs')
for t in ['K14C11_ABI_VERSION','ReviewedRegister','ReviewedProfileRegisters','IpBaseMap','REVIEWED_REGISTER_PROFILES','NAVI21_REVIEWED','NAVI48_REVIEWED','resolve_byte_offset','[C11RF]','[C11BA]','[C11HW]','[C11RD]']: assert t in c,t
for t in ['register_index:0x0da4','register_index:0x0dc1','register_index:0x0025','register_index:0x0024']: assert t in c,t
for t in ['RADEON_C11_MMIO_WRITES_ALLOWED: bool = false','RADEON_C11_FIRMWARE_UPLOAD_ALLOWED: bool = false','RADEON_C11_COMMAND_SUBMIT_ALLOWED: bool = false','RADEON_C11_BUS_MASTER_ALLOWED: bool = false']: assert t in c,t
assert 'bar_plus_raw_index=false' in c
assert 'IpBaseMap::EMPTY' in c
assert 'SYS_NATIVE_GPU_C11_QUERY: u64 = 22' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C11_QUERY, 22' in text('userspace/include/twabi.inc')
sys=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C11_QUERY' in sys and 'native_gpu_c11::packed_status()' in sys
d=text('userspace/displayd/displayd.S')
for t in ['K14.C11 reviewed Radeon register definitions and IP-base resolver gate online','no physical Radeon in QEMU','trusted AMD IP-base map is still required']: assert t in d,t
p=text('kernel/weavecore/src/process.rs')
for t in ['[KERN] K14.C11 alive:','[QUAL] K14.C11 reviewed-register runtime reached intentional post-userspace halt']: assert t in p,t
for f in ['K14C11_IMPLEMENTATION.md','K14C11_TESTER_GUIDE.md']: assert (root/f).exists(),f
print('Titanweave K14.C11 reviewed-register/IP-base source checks passed.')
