#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
c=text('kernel/weavecore/src/native_gpu_c21.rs')
for x in [
 'GFX12_SCRATCH_REG0_DWORD: u32 = 0x2040',
 'GFX12_SCRATCH_REG0_BASE_IDX: u8 = 1',
 'pub fn resolve_gfx12_scratch_reg0',
 'native_gpu_c20::resolve_verified_snapshot',
 'native_gpu_c19::with_verified_snapshot(resolve_gfx12_scratch_reg0)',
 'gc_segment1_base_dwords',
 'target_dword_offset',
 'target_byte_offset',
 'RADEON_C21_IDENTITY_MMIO_WRITE_ALLOWED: bool = true',
 'RADEON_C21_ARBITRARY_MMIO_WRITES_ALLOWED: bool = false',
 'RADEON_C21_MM_INDEX_FALLBACK_ALLOWED: bool = false',
 'RADEON_C21_BAR_RESIZE_ALLOWED: bool = false',
 'RADEON_C21_FIRMWARE_UPLOAD_ALLOWED: bool = false',
 'RADEON_C21_COMMAND_SUBMIT_ALLOWED: bool = false',
 'RADEON_C21_BUS_MASTER_ALLOWED: bool = false',
 'read_volatile', 'write_volatile',
 '[C21RV]', '[C21PG]', '[C21TX]', '[C21HW]', '[C21RD]'
]: assert x in c,x
c16=text('kernel/weavecore/src/native_gpu_c16.rs')
assert 'exact_generated_index_imported=false' in c16
m=text('kernel/weavecore/src/main.rs')
for x in ['mod native_gpu_c21;','native_gpu_c21::initialize(&mut allocator','[C21OK] K14.C21 reviewed GFX12 target rebind/identity-write gate:']: assert x in m,x
s=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C21_QUERY' in s and 'native_gpu_c21::packed_status()' in s
assert 'SYS_NATIVE_GPU_C21_QUERY: u64 = 32' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C21_QUERY, 32' in text('userspace/include/twabi.inc')
d=text('userspace/displayd/displayd.S')
for x in ['K14.C21 reviewed GFX12 SCRATCH_REG0 rebind/identity-write gate online','K14.C21 no physical Radeon in QEMU; reviewed GFX12 identity-write remains safely deferred','TW_SYS_NATIVE_GPU_C21_QUERY']: assert x in d,x
p=text('kernel/weavecore/src/process.rs')
for x in ['[KERN] K14.C21 alive:','[QUAL] K14.C21 reviewed-MMIO-rebind runtime reached intentional post-userspace halt']: assert x in p,x
r=text('tools/run-k14c21-qemu-reviewed-mmio-rebind.sh'); assert 'DISPLAY_BACKEND="${K13_DISPLAY:-gtk}"' in r
print('Titanweave K14.C21 reviewed GFX12 MMIO-rebind source checks passed.')
