#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
main=text('kernel/weavecore/src/main.rs')
for x in ['mod native_gpu_c12;','native_gpu_c12::initialize(&mut allocator, boot_info.bootstrap.page_table_root)','[C12OK] K14.C12 trusted IP-base/live-read engine:']: assert x in main,x
c=text('kernel/weavecore/src/native_gpu_c12.rs')
for x in ['K14C12_ABI_VERSION','RADEON_C12_MMIO_BAR_INDEX: u8 = 5','TrustedIpBaseMap','NAVI21_GC_BASE0_DWORDS','NAVI21_SDMA0_BASE0_DWORDS','map_from_discovery_records','read_ro_u32_page','[C12BS]','[C12RM]','[C12LR]','[C12HW]','[C12RD]','[C12R0]']: assert x in c,x
for x in ['RADEON_C12_MMIO_WRITES_ALLOWED: bool = false','RADEON_C12_FIRMWARE_UPLOAD_ALLOWED: bool = false','RADEON_C12_COMMAND_SUBMIT_ALLOWED: bool = false','RADEON_C12_BUS_MASTER_ALLOWED: bool = false']: assert x in c,x
assert 'SYS_NATIVE_GPU_C12_QUERY: u64 = 23' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C12_QUERY, 23' in text('userspace/include/twabi.inc')
s=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C12_QUERY' in s and 'native_gpu_c12::packed_status()' in s
d=text('userspace/displayd/displayd.S')
for x in ['K14.C12 trusted Radeon IP-base and live status-read gate online','no physical Radeon in QEMU','first live Radeon status reads verified']: assert x in d,x
p=text('kernel/weavecore/src/process.rs')
for x in ['[KERN] K14.C12 alive:','[QUAL] K14.C12 trusted-base/live-read runtime reached intentional post-userspace halt']: assert x in p,x
for f in ['K14C12_IMPLEMENTATION.md','K14C12_TESTER_GUIDE.md']: assert (root/f).exists(),f
print('Titanweave K14.C12 trusted-IP-base/live-read source checks passed.')
