#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
main=text('kernel/weavecore/src/main.rs')
for t in ['mod native_gpu_c5;','native_gpu_c5::initialize(&mut allocator)','[C5NF] K14.C5 AMD-Vi page-table engine ready:']: assert t in main,t
c5=text('kernel/weavecore/src/native_gpu_c5.rs')
for t in ['K14C5_ABI_VERSION','RADEON_C5_DOMAIN_ID','AmdViDomainImage::allocate','install_exact_requester_dte','persistent_domain_live','[C5PT]','[C5CB]','[C5HW]','[C5RD]']: assert t in c5,t
amd=text('kernel/weavecore/src/amd_vi.rs')
for t in ['AMDVI_DTE_BYTES','AMDVI_COMMAND_BYTES','AMDVI_EVENT_BYTES','AMDVI_DEVICE_TABLE_PAGES','AmdViDomainImage','c5_layout_self_test']: assert t in amd,t
abi=text('kernel/weavecore/src/abi.rs'); assert 'SYS_NATIVE_GPU_C5_QUERY: u64 = 16' in abi
inc=text('userspace/include/twabi.inc'); assert 'TW_SYS_NATIVE_GPU_C5_QUERY, 16' in inc
sys=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C5_QUERY' in sys and 'native_gpu_c5::packed_status()' in sys
d=text('userspace/displayd/displayd.S')
for t in ['K14.C5 AMD-Vi page-table engine foundation online','no physical Radeon in QEMU','hardware register programming remains fenced']: assert t in d,t
p=text('kernel/weavecore/src/process.rs')
for t in ['[KERN] K14.C5 alive:','[QUAL] K14.C5 AMD-Vi page-table runtime reached intentional post-userspace halt']: assert t in p,t
for f in ['K14C5_IMPLEMENTATION.md','K14C5_TESTER_GUIDE.md']: assert (root/f).exists(),f
print('Titanweave K14.C5 AMD-Vi page-table source checks passed.')
