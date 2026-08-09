#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
main=text('kernel/weavecore/src/main.rs')
for t in ['mod native_gpu_c9;','native_gpu_c9::initialize()','[C9NF] K14.C9 verified Radeon profiles ready:']: assert t in main,t
c9=text('kernel/weavecore/src/native_gpu_c9.rs')
for t in ['K14C9_ABI_VERSION','VERIFIED_RADEON_PROFILES','0x73bf','0x7550','Navi21Rx6800_6900','Navi48Rx9070','RADEON_C9_SAFE_PCI_READS','pci::read_u32','pci::read_u16','command_bus_master_seen','mmio_whitelist_entries: 0','RADEON_C9_MMIO_READS_ALLOWED: bool = false','[C9PF]','[C9PR]','[C9IP]','[C9HW]','[C9RD]']: assert t in c9,t
assert 'SYS_NATIVE_GPU_C9_QUERY: u64 = 20' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C9_QUERY, 20' in text('userspace/include/twabi.inc')
sys=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C9_QUERY' in sys and 'native_gpu_c9::packed_status()' in sys
d=text('userspace/displayd/displayd.S')
for t in ['K14.C9 verified Radeon profiles and live safe-identity-read gate online','no physical Radeon in QEMU','live PCI identity reads verified the Radeon profile']: assert t in d,t
p=text('kernel/weavecore/src/process.rs')
for t in ['[KERN] K14.C9 alive:','[QUAL] K14.C9 verified Radeon profile runtime reached intentional post-userspace halt']: assert t in p,t
for f in ['K14C9_IMPLEMENTATION.md','K14C9_TESTER_GUIDE.md']: assert (root/f).exists(),f
print('Titanweave K14.C9 verified Radeon profile/live safe-read source checks passed.')
