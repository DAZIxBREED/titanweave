#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
c=text('kernel/weavecore/src/native_gpu_c23.rs')
for x in [
 'K14C23_ABI_VERSION: u32 = 1',
 'derive_distinct_one_bit_probe',
 'native_gpu_c22::derive_one_bit_probe',
 'RADEON_C23_DUAL_PROBE_ALLOWED: bool = true',
 'RADEON_C23_MAX_MUTATION_POLLS_PER_CYCLE: u8 = 32',
 'RADEON_C23_MAX_RESTORE_POLLS_PER_CYCLE: u8 = 32',
 'RADEON_C23_MAX_MMIO_WRITES: u8 = 6',
 'RADEON_C23_ARBITRARY_MMIO_WRITES_ALLOWED: bool = false',
 'RADEON_C23_CALLER_SUPPLIED_VALUE_ALLOWED: bool = false',
 'RADEON_C23_CALLER_SUPPLIED_ADDRESS_ALLOWED: bool = false',
 'RADEON_C23_MM_INDEX_FALLBACK_ALLOWED: bool = false',
 'RADEON_C23_BAR_RESIZE_ALLOWED: bool = false',
 'RADEON_C23_FIRMWARE_UPLOAD_ALLOWED: bool = false',
 'RADEON_C23_COMMAND_SUBMIT_ALLOWED: bool = false',
 'RADEON_C23_BUS_MASTER_ALLOWED: bool = false',
 'native_gpu_c22::state()',
 'native_gpu_c19::with_verified_snapshot(native_gpu_c21::resolve_gfx12_scratch_reg0)',
 'c22_restore_persisted', 'intercycle_restore_persisted',
 'cycle_a_mutation_verified', 'cycle_b_mutation_verified',
 'write_volatile', 'read_volatile',
 '[C23PS]', '[C23PG]', '[C23TX]', '[C23HW]', '[C23RD]'
]: assert x in c,x
m=text('kernel/weavecore/src/main.rs')
for x in ['mod native_gpu_c23;','native_gpu_c23::initialize(&mut allocator','[C23OK] K14.C23 GFX12 SCRATCH_REG0 persistence/dual-probe stability gate:']: assert x in m,x
s=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C23_QUERY' in s and 'native_gpu_c23::packed_status()' in s
assert 'SYS_NATIVE_GPU_C23_QUERY: u64 = 34' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C23_QUERY, 34' in text('userspace/include/twabi.inc')
d=text('userspace/displayd/displayd.S')
for x in ['K14.C23 GFX12 SCRATCH_REG0 persistence and dual-probe stability gate online','K14.C23 no physical Radeon in QEMU; dual-probe stability transaction remains safely deferred','TW_SYS_NATIVE_GPU_C23_QUERY','test eax, 0x2000']: assert x in d,x
p=text('kernel/weavecore/src/process.rs')
for x in ['[KERN] K14.C23 alive:','[QUAL] K14.C23 dual-probe-stability runtime reached intentional post-userspace halt']: assert x in p,x
r=text('tools/run-k14c23-qemu-dual-probe-stability.sh')
for x in ['DISPLAY_BACKEND="${K13_DISPLAY:-gtk}"', "HALT_MARKER='[HALT] BSP halted intentionally'", 'Intentional Titanweave HALT detected; terminating QEMU.']: assert x in r,x
print('Titanweave K14.C23 dual-probe stability source checks passed.')
