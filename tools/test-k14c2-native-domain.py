#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
main=text('kernel/weavecore/src/main.rs')
for t in ['mod native_gpu_c2;','native_gpu_c2::initialize(&mut allocator','[C2NF] K14.C2 native persistent-domain/AMD bring-up ready:']: assert t in main,t
tr=text('kernel/weavecore/src/translated_dma.rs')
for t in ['PersistentDomainQualification','qualify_persistent_domain_surrogate','[PDOM] K14.C2 persistent translated-domain surrogate:','[PDRV] K14.C2 persistent-domain surrogate revoked:','epochs >= 3' if False else 'for epoch in 0..3u32']: assert t in tr,t
c2=text('kernel/weavecore/src/native_gpu_c2.rs')
for t in ['AMD_REQUIRED_FIRMWARE_MASK','AMD_BOOTSTRAP_RING_ENTRIES','actual_gpu_domain_bound = false','vendor_mmio_writes: false','bus_master_enabled: false','[AFWP]','[ARING]','[ASIC]','[C2RD]']: assert t in c2,t
assert 'pci::enable_bus_master' not in c2
abi=text('kernel/weavecore/src/abi.rs'); assert 'SYS_NATIVE_GPU_C2_QUERY: u64 = 13' in abi
inc=text('userspace/include/twabi.inc'); assert 'TW_SYS_NATIVE_GPU_C2_QUERY, 13' in inc
sys=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C2_QUERY' in sys and 'native_gpu_c2::packed_status()' in sys
d=text('userspace/displayd/displayd.S');
for t in ['TW_SYS_NATIVE_GPU_C2_QUERY','K14.C2 persistent-domain and AMD bring-up contract online','physical Radeon domain remains fenced']: assert t in d,t
p=text('kernel/weavecore/src/process.rs');
for t in ['[KERN] K14.C2 alive:','[QUAL] K14.C2 native bring-up runtime reached intentional post-userspace halt']: assert t in p,t
for f in ['K14C2_IMPLEMENTATION.md','K14C2_TESTER_GUIDE.md']: assert (root/f).exists(),f
print('Titanweave K14.C2 persistent-domain/AMD bring-up source checks passed.')
