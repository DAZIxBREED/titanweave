#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
c=text('kernel/weavecore/src/native_gpu_c26.rs')
for x in [
 'K14C26_ABI_VERSION: u32 = 1',
 'GFX12_SCRATCH_REG1_DWORD: u32 = 0x2041',
 'GFX12_SCRATCH_REG1_BASE_IDX: u8 = 1',
 'RADEON_C26_ALLOWLIST_ENTRY_COUNT: u8 = 2',
 'ReviewedAccess::FrozenReversibleProbe', 'ReviewedAccess::ReadOnly',
 'GFX12_REVIEWED_MMIO_ALLOWLIST',
 'RADEON_C26_REG1_READ_ALLOWED: bool = true',
 'RADEON_C26_REG1_WRITE_ALLOWED: bool = false',
 'RADEON_C26_MAX_MMIO_WRITES: u8 = 0',
 'RADEON_C26_ARBITRARY_MMIO_WRITES_ALLOWED: bool = false',
 'RADEON_C26_CALLER_SUPPLIED_VALUE_ALLOWED: bool = false',
 'RADEON_C26_CALLER_SUPPLIED_ADDRESS_ALLOWED: bool = false',
 'RADEON_C26_MM_INDEX_FALLBACK_ALLOWED: bool = false',
 'RADEON_C26_BAR_RESIZE_ALLOWED: bool = false',
 'RADEON_C26_FIRMWARE_UPLOAD_ALLOWED: bool = false',
 'RADEON_C26_COMMAND_SUBMIT_ALLOWED: bool = false',
 'RADEON_C26_BUS_MASTER_ALLOWED: bool = false',
 'K14C26_FINAL_K14_COMPLETION_GATE: bool = true',
 'resolve_gfx12_scratch_reg1',
 'native_gpu_c21::resolve_gfx12_scratch_reg0',
 'native_gpu_c19::with_verified_snapshot(resolve_gfx12_scratch_reg1)',
 'read_volatile', 'writes_performed: 0', 'no_write_verified',
 'targets_distinct', 'targets_adjacent', 'allowlist_exact',
 '[C26RV]', '[C26AL]', '[C26PG]', '[C26HW]', '[C26RD]'
]: assert x in c,x
assert 'write_volatile' not in c, 'C26 must contain no MMIO write_volatile path'
m=text('kernel/weavecore/src/main.rs')
for x in ['mod native_gpu_c26;','native_gpu_c26::initialize(&mut allocator','[C26OK] K14.C26 final reviewed GFX12 MMIO allowlist/read-only completion gate:']: assert x in m,x
s=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C26_QUERY' in s and 'native_gpu_c26::packed_status()' in s
assert 'SYS_NATIVE_GPU_C26_QUERY: u64 = 37' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C26_QUERY, 37' in text('userspace/include/twabi.inc')
d=text('userspace/displayd/displayd.S')
for x in ['K14.C26 final GFX12 reviewed MMIO allowlist and SCRATCH_REG1 read-only completion gate online','K14.C26 no physical Radeon in QEMU; final K14 SCRATCH_REG1 read proof remains safely deferred','TW_SYS_NATIVE_GPU_C26_QUERY','test eax, 0x2000']: assert x in d,x
p=text('kernel/weavecore/src/process.rs')
for x in ['[KERN] K14.C26 alive:','[QUAL] K14.C26 final-k14-mmio-allowlist runtime reached intentional post-userspace halt','[K14FOUND] K14.C26 native Radeon MMIO foundation frozen; fixed roadmap continues K14 Radeon through C32 before K15 ForgeAudio']: assert x in p,x
q=text('K14C26_RUNTIME_QUALIFICATION.md'); assert '[K14DONE] K14 native Radeon foundation completion gate reached; broader driver bring-up moves to K15' in q
rr=text('tools/run-k14c26-qemu-final-mmio-allowlist.sh')
for x in ['DISPLAY_BACKEND="${K13_DISPLAY:-gtk}"', "HALT_MARKER='[HALT] BSP halted intentionally'", 'Intentional Titanweave HALT detected; terminating QEMU.']: assert x in rr,x
print('Titanweave K14.C26 final MMIO allowlist source checks passed.')
