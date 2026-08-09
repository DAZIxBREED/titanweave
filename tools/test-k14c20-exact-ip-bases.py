#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
c=text('kernel/weavecore/src/native_gpu_c20.rs')
for x in [
 'AMD_GC_HWID: u16 = 11','AMD_SDMA0_HWID: u16 = 42','AMD_SDMA1_HWID: u16 = 43',
 'AMD_SDMA2_HWID: u16 = 68','AMD_SDMA3_HWID: u16 = 69',
 'pub fn resolve_verified_snapshot','native_gpu_c17::parse_discovery_snapshot',
 '0x3fff_ffff','RADEON_C20_MMIO_WRITES_ALLOWED: bool = false',
 'RADEON_C20_FIRMWARE_UPLOAD_ALLOWED: bool = false','RADEON_C20_COMMAND_SUBMIT_ALLOWED: bool = false',
 'RADEON_C20_BUS_MASTER_ALLOWED: bool = false','[C20IP]','[C20PG]','[C20HW]','[C20RD]'
]: assert x in c,x
m=text('kernel/weavecore/src/main.rs')
for x in ['mod native_gpu_c20;','native_gpu_c20::initialize()','[C20OK] K14.C20 AMD exact live IP-base gate:']: assert x in m,x
s=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C20_QUERY' in s and 'native_gpu_c20::packed_status()' in s
assert 'SYS_NATIVE_GPU_C20_QUERY: u64 = 31' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C20_QUERY, 31' in text('userspace/include/twabi.inc')
d=text('userspace/displayd/displayd.S')
for x in ['K14.C20 AMD exact live IP-base resolver online','K14.C20 no physical Radeon in QEMU; verified-snapshot IP-base resolution remains safely deferred','TW_SYS_NATIVE_GPU_C20_QUERY']: assert x in d,x
p=text('kernel/weavecore/src/process.rs')
for x in ['[KERN] K14.C20 alive:','[QUAL] K14.C20 exact-IP-base runtime reached intentional post-userspace halt']: assert x in p,x
print('Titanweave K14.C20 exact AMD IP-base source checks passed.')
