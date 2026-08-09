#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
main=text('kernel/weavecore/src/main.rs')
for t in ['mod native_gpu_c4;','native_gpu_c4::initialize()','[C4NF] K14.C4 Radeon exact-domain qualification ready:']: assert t in main,t
c4=text('kernel/weavecore/src/native_gpu_c4.rs')
for t in ['K14C4_ABI_VERSION','RequesterId','AMD_VI_REQUIRED_FOR_AMD_HOST','requester_domain_planned','persistent_domain_live','mmio_write_allowed: false','firmware_upload_allowed: false','command_submit_allowed: false','bus_master_enabled: false','[C4IV]','[C4HW]','[C4RD]']: assert t in c4,t
assert 'hardware_amd_vi_page_tables_not_yet_live' in c4
sys=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C4_QUERY' in sys and 'native_gpu_c4::packed_status()' in sys
abi=text('kernel/weavecore/src/abi.rs'); assert 'SYS_NATIVE_GPU_C4_QUERY: u64 = 15' in abi
inc=text('userspace/include/twabi.inc'); assert 'TW_SYS_NATIVE_GPU_C4_QUERY, 15' in inc
d=text('userspace/displayd/displayd.S')
for t in ['K14.C4 exact Radeon requester/AMD-Vi gate online','no physical Radeon in QEMU','destructive bring-up remains fenced']: assert t in d,t
p=text('kernel/weavecore/src/process.rs')
for t in ['[KERN] K14.C4 alive:','[QUAL] K14.C4 Radeon domain-gate runtime reached intentional post-userspace halt']: assert t in p,t
for f in ['K14C4_IMPLEMENTATION.md','K14C4_TESTER_GUIDE.md']: assert (root/f).exists(),f
print('Titanweave K14.C4 Radeon exact-domain source checks passed.')
