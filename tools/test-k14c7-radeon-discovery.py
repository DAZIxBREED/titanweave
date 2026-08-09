#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
main=text('kernel/weavecore/src/main.rs')
for t in ['mod native_gpu_c7;','native_gpu_c7::initialize(&mut allocator, boot_info.bootstrap.page_table_root)','[C7NF] K14.C7 Radeon discovery ready:']: assert t in main,t
c7=text('kernel/weavecore/src/native_gpu_c7.rs')
for t in ['K14C7_ABI_VERSION','RADEON_C7_PROBE_BYTES','map_kernel_mmio_readonly','exact_domain_live','firmware_manifest_ready','gmc_gtt_readiness_planned','[C7AP]','[C7FW]','[C7GT]','[C7HW]','[C7RD]']: assert t in c7,t
paging=text('kernel/weavecore/src/paging.rs')
for t in ['map_kernel_mmio_readonly','if writable { flags |= PAGE_WRITABLE; }']: assert t in paging,t
assert 'SYS_NATIVE_GPU_C7_QUERY: u64 = 18' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C7_QUERY, 18' in text('userspace/include/twabi.inc')
sys=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C7_QUERY' in sys and 'native_gpu_c7::packed_status()' in sys
d=text('userspace/displayd/displayd.S')
for t in ['K14.C7 Radeon MMIO/firmware discovery staging online','no physical Radeon in QEMU','supervisor read-only Radeon aperture mapped']: assert t in d,t
p=text('kernel/weavecore/src/process.rs')
for t in ['[KERN] K14.C7 alive:','[QUAL] K14.C7 Radeon discovery runtime reached intentional post-userspace halt']: assert t in p,t
for f in ['K14C7_IMPLEMENTATION.md','K14C7_TESTER_GUIDE.md']: assert (root/f).exists(),f
print('Titanweave K14.C7 Radeon discovery source checks passed.')
