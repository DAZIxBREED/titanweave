#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
c=text('kernel/weavecore/src/native_gpu_c22.rs')
for x in [
 'K14C22_ABI_VERSION: u32 = 1',
 'derive_one_bit_probe',
 'RADEON_C22_ONE_BIT_SCRATCH_MUTATION_ALLOWED: bool = true',
 'RADEON_C22_MAX_MUTATION_POLLS: u8 = 32',
 'RADEON_C22_MAX_RESTORE_POLLS: u8 = 32',
 'RADEON_C22_MAX_MMIO_WRITES: u8 = 3',
 'RADEON_C22_ARBITRARY_MMIO_WRITES_ALLOWED: bool = false',
 'RADEON_C22_CALLER_SUPPLIED_VALUE_ALLOWED: bool = false',
 'RADEON_C22_CALLER_SUPPLIED_ADDRESS_ALLOWED: bool = false',
 'RADEON_C22_MM_INDEX_FALLBACK_ALLOWED: bool = false',
 'RADEON_C22_BAR_RESIZE_ALLOWED: bool = false',
 'RADEON_C22_FIRMWARE_UPLOAD_ALLOWED: bool = false',
 'RADEON_C22_COMMAND_SUBMIT_ALLOWED: bool = false',
 'RADEON_C22_BUS_MASTER_ALLOWED: bool = false',
 'native_gpu_c21::state()',
 'native_gpu_c19::with_verified_snapshot(native_gpu_c21::resolve_gfx12_scratch_reg0)',
 'write_volatile', 'read_volatile',
 'restore_retry_used', '[C22RV]', '[C22PG]', '[C22TX]', '[C22HW]', '[C22RD]'
]: assert x in c,x
m=text('kernel/weavecore/src/main.rs')
for x in ['mod native_gpu_c22;','native_gpu_c22::initialize(&mut allocator','[C22OK] K14.C22 reversible GFX12 SCRATCH_REG0 mutation gate:']: assert x in m,x
s=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C22_QUERY' in s and 'native_gpu_c22::packed_status()' in s
assert 'SYS_NATIVE_GPU_C22_QUERY: u64 = 33' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C22_QUERY, 33' in text('userspace/include/twabi.inc')
d=text('userspace/displayd/displayd.S')
for x in ['K14.C22 bounded reversible GFX12 SCRATCH_REG0 mutation gate online','K14.C22 no physical Radeon in QEMU; reversible scratch mutation remains safely deferred','TW_SYS_NATIVE_GPU_C22_QUERY']: assert x in d,x
p=text('kernel/weavecore/src/process.rs')
for x in ['[KERN] K14.C22 alive:','[QUAL] K14.C22 reversible-scratch-mutation runtime reached intentional post-userspace halt']: assert x in p,x
r=text('tools/run-k14c22-qemu-reversible-scratch-mutation.sh')
for x in ['DISPLAY_BACKEND="${K13_DISPLAY:-gtk}"','HALT_MARKER=\'[HALT] BSP halted intentionally\'','Intentional Titanweave HALT detected; terminating QEMU.']: assert x in r,x
print('Titanweave K14.C22 reversible scratch-mutation source checks passed.')
