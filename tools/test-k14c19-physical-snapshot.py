#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
c=text('kernel/weavecore/src/native_gpu_c19.rs')
for x in [
 'K14C19_ABI_VERSION: u32 = 1',
 'RADEON_C19_DIRECT_VRAM_APERTURE_READ_ALLOWED: bool = true',
 'RADEON_C19_MM_INDEX_FALLBACK_ALLOWED: bool = false',
 'RADEON_C19_MMIO_WRITES_ALLOWED: bool = false',
 'RADEON_C19_FIRMWARE_UPLOAD_ALLOWED: bool = false',
 'RADEON_C19_COMMAND_SUBMIT_ALLOWED: bool = false',
 'RADEON_C19_BUS_MASTER_ALLOWED: bool = false',
 'PCI_EXT_CAP_ID_REBAR: u16 = 0x15',
 'PCI_REBAR_CTRL_BAR_SIZE_MASK: u32 = 0x0000_1f00',
 'bar0_rebar_size', 'resolve_tmr', 'read_bar5_u32', 'ecam_config_mapping',
 'copy_vram_snapshot', 'verify_snapshot_checksums', 'with_verified_snapshot',
 '[C19PG] physical discovery-read policy:', '[C19AP] VRAM aperture contract:',
 '[C19HW] physical AMD discovery snapshot:', '[C19RD] K14.C19 physical snapshot gate ready:'
]: assert x in c, x
m=text('kernel/weavecore/src/main.rs')
for x in ['mod native_gpu_c19;','native_gpu_c19::initialize(','[C19OK] K14.C19 physical AMD discovery snapshot gate:']: assert x in m,x
assert 'SYS_NATIVE_GPU_C19_QUERY: u64 = 30' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C19_QUERY, 30' in text('userspace/include/twabi.inc')
s=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C19_QUERY' in s and 'native_gpu_c19::packed_status()' in s
d=text('userspace/displayd/displayd.S')
for x in ['K14.C19 physical AMD discovery snapshot gate online','K14.C19 no physical Radeon in QEMU; direct BAR0 discovery snapshot read remains safely deferred','TW_SYS_NATIVE_GPU_C19_QUERY']: assert x in d,x
p=text('kernel/weavecore/src/process.rs')
for x in ['[KERN] K14.C19 alive:','[QUAL] K14.C19 physical-snapshot runtime reached intentional post-userspace halt']: assert x in p,x
assert (root/'K14C19_IMPLEMENTATION.md').is_file()
assert (root/'K14C19_TESTER_GUIDE.md').is_file()
print('Titanweave K14.C19 physical AMD discovery snapshot source checks passed.')
