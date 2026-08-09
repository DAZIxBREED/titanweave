#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
c=text('kernel/weavecore/src/native_gpu_c24.rs')
for x in [
 'K14C24_ABI_VERSION: u32 = 1',
 'derive_four_bit_pattern', '0x0000_000f', '0x0000_00f0',
 'RADEON_C24_PATTERN_BITS: u32 = 4',
 'RADEON_C24_MULTI_BIT_PATTERN_ALLOWED: bool = true',
 'RADEON_C24_MAX_PATTERN_POLLS: u8 = 32',
 'RADEON_C24_MAX_RESTORE_POLLS: u8 = 32',
 'RADEON_C24_MAX_MMIO_WRITES: u8 = 3',
 'RADEON_C24_ARBITRARY_MMIO_WRITES_ALLOWED: bool = false',
 'RADEON_C24_CALLER_SUPPLIED_VALUE_ALLOWED: bool = false',
 'RADEON_C24_CALLER_SUPPLIED_ADDRESS_ALLOWED: bool = false',
 'RADEON_C24_MM_INDEX_FALLBACK_ALLOWED: bool = false',
 'RADEON_C24_BAR_RESIZE_ALLOWED: bool = false',
 'RADEON_C24_FIRMWARE_UPLOAD_ALLOWED: bool = false',
 'RADEON_C24_COMMAND_SUBMIT_ALLOWED: bool = false',
 'RADEON_C24_BUS_MASTER_ALLOWED: bool = false',
 'native_gpu_c23::state()', 'c23.dual_cycle_verified',
 'native_gpu_c19::with_verified_snapshot(native_gpu_c21::resolve_gfx12_scratch_reg0)',
 'c23_restore_persisted', 'pattern_verified', 'restore_verified',
 'write_volatile', 'read_volatile',
 '[C24PT]', '[C24PG]', '[C24TX]', '[C24HW]', '[C24RD]'
]: assert x in c,x
m=text('kernel/weavecore/src/main.rs')
for x in ['mod native_gpu_c24;','native_gpu_c24::initialize(&mut allocator','[C24OK] K14.C24 reversible GFX12 SCRATCH_REG0 multi-bit pattern gate:']: assert x in m,x
s=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C24_QUERY' in s and 'native_gpu_c24::packed_status()' in s
assert 'SYS_NATIVE_GPU_C24_QUERY: u64 = 35' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C24_QUERY, 35' in text('userspace/include/twabi.inc')
d=text('userspace/displayd/displayd.S')
for x in ['K14.C24 reversible GFX12 SCRATCH_REG0 multi-bit pattern gate online','K14.C24 no physical Radeon in QEMU; reversible multi-bit pattern transaction remains safely deferred','TW_SYS_NATIVE_GPU_C24_QUERY','test eax, 0x2000']: assert x in d,x
p=text('kernel/weavecore/src/process.rs')
for x in ['[KERN] K14.C24 alive:','[QUAL] K14.C24 reversible-multi-bit-pattern runtime reached intentional post-userspace halt']: assert x in p,x
r=text('tools/run-k14c24-qemu-multi-bit-pattern.sh')
for x in ['DISPLAY_BACKEND="${K13_DISPLAY:-gtk}"', "HALT_MARKER='[HALT] BSP halted intentionally'", 'Intentional Titanweave HALT detected; terminating QEMU.']: assert x in r,x
print('Titanweave K14.C24 reversible multi-bit pattern source checks passed.')
