#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
main=text('kernel/weavecore/src/main.rs')
for t in ['mod native_gpu_c8;','native_gpu_c8::initialize()','[C8NF] K14.C8 Radeon ASIC/IP identification ready:']: assert t in main,t
c8=text('kernel/weavecore/src/native_gpu_c8.rs')
for t in ['K14C8_ABI_VERSION','VerifiedAsicProfile','SafeRegisterDescriptor','VERIFIED_ASIC_PROFILES','read_has_side_effects','safe_read_whitelist_ready','firmware_requirements_resolved','gmc_gtt_init_ready','[C8ID]','[C8RR]','[C8IP]','[C8HW]','[C8RD]']: assert t in c8,t
assert 'static VERIFIED_ASIC_PROFILES: &[VerifiedAsicProfile] = &[];' in c8
assert 'SYS_NATIVE_GPU_C8_QUERY: u64 = 19' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C8_QUERY, 19' in text('userspace/include/twabi.inc')
sys=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C8_QUERY' in sys and 'native_gpu_c8::packed_status()' in sys
d=text('userspace/displayd/displayd.S')
for t in ['K14.C8 Radeon ASIC/IP identification and safe-register-read gate online','no physical Radeon in QEMU','ASIC profile is unverified']: assert t in d,t
p=text('kernel/weavecore/src/process.rs')
for t in ['[KERN] K14.C8 alive:','[QUAL] K14.C8 Radeon ASIC/IP runtime reached intentional post-userspace halt']: assert t in p,t
for f in ['K14C8_IMPLEMENTATION.md','K14C8_TESTER_GUIDE.md']: assert (root/f).exists(),f
print('Titanweave K14.C8 Radeon ASIC/IP source checks passed.')
