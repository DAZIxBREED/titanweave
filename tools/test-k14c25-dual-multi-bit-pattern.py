#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
c=text('kernel/weavecore/src/native_gpu_c25.rs')
for x in [
 'K14C25_ABI_VERSION: u32 = 1',
 'derive_pattern_a', 'derive_pattern_b',
 '0x0000_000f', '0x0000_0f00', '0x0000_00f0', '0x0000_f000',
 'RADEON_C25_PATTERN_BITS_PER_CYCLE: u32 = 4',
 'RADEON_C25_DUAL_MULTI_BIT_PATTERN_ALLOWED: bool = true',
 'RADEON_C25_MAX_PATTERN_POLLS_PER_CYCLE: u8 = 32',
 'RADEON_C25_MAX_RESTORE_POLLS_PER_CYCLE: u8 = 32',
 'RADEON_C25_MAX_MMIO_WRITES: u8 = 6',
 'RADEON_C25_ARBITRARY_MMIO_WRITES_ALLOWED: bool = false',
 'RADEON_C25_CALLER_SUPPLIED_VALUE_ALLOWED: bool = false',
 'RADEON_C25_CALLER_SUPPLIED_ADDRESS_ALLOWED: bool = false',
 'RADEON_C25_MM_INDEX_FALLBACK_ALLOWED: bool = false',
 'RADEON_C25_BAR_RESIZE_ALLOWED: bool = false',
 'RADEON_C25_FIRMWARE_UPLOAD_ALLOWED: bool = false',
 'RADEON_C25_COMMAND_SUBMIT_ALLOWED: bool = false',
 'RADEON_C25_BUS_MASTER_ALLOWED: bool = false',
 'native_gpu_c24::state()', 'c24.pattern_verified', 'c24.restore_verified',
 'native_gpu_c19::with_verified_snapshot(native_gpu_c21::resolve_gfx12_scratch_reg0)',
 'c24_restore_persisted', 'intercycle_restore_persisted', 'dual_pattern_verified',
 'write_volatile', 'read_volatile',
 '[C25DP]', '[C25PG]', '[C25TX]', '[C25HW]', '[C25RD]'
]: assert x in c,x
m=text('kernel/weavecore/src/main.rs')
for x in ['mod native_gpu_c25;','native_gpu_c25::initialize(&mut allocator','[C25OK] K14.C25 GFX12 SCRATCH_REG0 dual multi-bit pattern stability gate:']: assert x in m,x
s=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C25_QUERY' in s and 'native_gpu_c25::packed_status()' in s
assert 'SYS_NATIVE_GPU_C25_QUERY: u64 = 36' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C25_QUERY, 36' in text('userspace/include/twabi.inc')
d=text('userspace/displayd/displayd.S')
for x in ['K14.C25 GFX12 SCRATCH_REG0 dual multi-bit pattern stability gate online','K14.C25 no physical Radeon in QEMU; dual multi-bit pattern stability transaction remains safely deferred','TW_SYS_NATIVE_GPU_C25_QUERY','test eax, 0x2000']: assert x in d,x
p=text('kernel/weavecore/src/process.rs')
for x in ['[KERN] K14.C25 alive:','[QUAL] K14.C25 dual-multi-bit-pattern runtime reached intentional post-userspace halt']: assert x in p,x
rr=text('tools/run-k14c25-qemu-dual-multi-bit-pattern.sh')
for x in ['DISPLAY_BACKEND="${K13_DISPLAY:-gtk}"', "HALT_MARKER='[HALT] BSP halted intentionally'", 'Intentional Titanweave HALT detected; terminating QEMU.']: assert x in rr,x
print('Titanweave K14.C25 dual multi-bit pattern source checks passed.')
