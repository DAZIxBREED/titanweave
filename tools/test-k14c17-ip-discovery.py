#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
c=text('kernel/weavecore/src/native_gpu_c17.rs');m=text('kernel/weavecore/src/main.rs')
for x in ['K14C17_ABI_VERSION','AMD_DISCOVERY_BINARY_SIGNATURE:u32=0x2821_1407','AMD_DISCOVERY_TABLE_SIGNATURE:u32=0x5344_5049','parse_discovery_snapshot','AMD_DISCOVERY_MAX_TABLES','AMD_DISCOVERY_MAX_DIES','parser_self_test','RADEON_C17_LIVE_VRAM_DISCOVERY_READ_ALLOWED:bool=false','RADEON_C17_MMIO_WRITE_ALLOWED:bool=false','guessed_bases=false','[C17IP]','[C17PG]','[C17HW]','[C17RD]']: assert x in c,x
for x in ['mod native_gpu_c17;','native_gpu_c17::initialize()','[C17OK] K14.C17 AMD IP-discovery/Navi48 base-resolution gate:']: assert x in m,x
assert 'SYS_NATIVE_GPU_C17_QUERY: u64 = 28' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C17_QUERY, 28' in text('userspace/include/twabi.inc')
s=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C17_QUERY' in s and 'native_gpu_c17::packed_status()' in s
d=text('userspace/displayd/displayd.S');
for x in ['K14.C17 AMD IP-discovery/Navi48 base-resolution gate online','no physical Radeon in QEMU','verified live discovery snapshot/exact GC base unresolved']: assert x in d,x
p=text('kernel/weavecore/src/process.rs');
for x in ['[KERN] K14.C17 alive:','[QUAL] K14.C17 IP-discovery runtime reached intentional post-userspace halt']: assert x in p,x
for q in ['K14C17_IMPLEMENTATION.md','K14C17_TESTER_GUIDE.md']: assert (root/q).exists(),q
print('Titanweave K14.C17 AMD IP-discovery source checks passed.')
