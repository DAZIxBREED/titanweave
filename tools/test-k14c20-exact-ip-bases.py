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
print('Titanweave K14.C20 exact AMD IP-base source checks passed.')
