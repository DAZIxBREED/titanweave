#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
main=text('kernel/weavecore/src/main.rs')
for x in ['mod native_gpu_c15;','native_gpu_c15::initialize()','[C15OK] K14.C15 controlled write transaction:']: assert x in main,x
c=text('kernel/weavecore/src/native_gpu_c15.rs')
for x in ['K14C15_ABI_VERSION','RADEON_C15_PCI_IDENTITY_WRITE_ALLOWED','[C15TX]','[C15RB]','[C15HW]','[C15RD]','identity_write_attempted','identity_write_verified','rollback_attempted']: assert x in c,x
for x in ['RADEON_C15_MMIO_WRITES_ALLOWED: bool = false','RADEON_C15_FIRMWARE_UPLOAD_ALLOWED: bool = false','RADEON_C15_COMMAND_SUBMIT_ALLOWED: bool = false','RADEON_C15_BUS_MASTER_ALLOWED: bool = false']: assert x in c,x
assert 'pci::write_u16(' in c
pci=text('kernel/weavecore/src/pci.rs')
assert 'pub fn write_u16(' in pci and 'outw(CONFIG_DATA+lane,value)' in pci
assert 'SYS_NATIVE_GPU_C15_QUERY: u64 = 26' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C15_QUERY, 26' in text('userspace/include/twabi.inc')
s=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C15_QUERY' in s and 'native_gpu_c15::packed_status()' in s
d=text('userspace/displayd/displayd.S')
for x in ['K14.C15 controlled Radeon write transaction gate online','no physical Radeon in QEMU','PCI Command identity-write/readback transaction qualified']: assert x in d,x
p=text('kernel/weavecore/src/process.rs')
for x in ['[KERN] K14.C15 alive:','[QUAL] K14.C15 controlled-write runtime reached intentional post-userspace halt']: assert x in p,x
for f in ['K14C15_IMPLEMENTATION.md','K14C15_TESTER_GUIDE.md']: assert (root/f).exists(),f
print('Titanweave K14.C15 controlled write-transaction source checks passed.')
