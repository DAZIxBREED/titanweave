#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
main=text('kernel/weavecore/src/main.rs')
for t in ['mod native_gpu_c3;','native_gpu_c3::initialize()','[C3NF] K14.C3 Radeon bare-metal staging ready:']: assert t in main,t
c3=text('kernel/weavecore/src/native_gpu_c3.rs')
for t in ['K14C3_ABI_VERSION','AMD_IP_BRINGUP_ORDER','AmdIpBlock::Psp','AmdIpBlock::DisplayCore','amd_vi_live','actual_gpu_domain_bound','command_submission_authorized: false','bus_master_enabled: false','[C3HW]','[C3RD]']: assert t in c3,t
assert 'state.actual_gpu_domain_bound' in c3 and 'state.mmio_mapping_authorized = true' in c3
assert 'firmware_upload_authorized = false' in c3 and 'command_submission_authorized = false' in c3
sys=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C3_QUERY' in sys and 'native_gpu_c3::packed_status()' in sys
abi=text('kernel/weavecore/src/abi.rs'); assert 'SYS_NATIVE_GPU_C3_QUERY: u64 = 14' in abi
inc=text('userspace/include/twabi.inc'); assert 'TW_SYS_NATIVE_GPU_C3_QUERY, 14' in inc
d=text('userspace/displayd/displayd.S')
for t in ['K14.C3 Radeon bare-metal staging contract online','no physical Radeon in QEMU','bus mastering remains blocked']: assert t in d,t
p=text('kernel/weavecore/src/process.rs')
for t in ['[KERN] K14.C3 alive:','[QUAL] K14.C3 Radeon staging runtime reached intentional post-userspace halt']: assert t in p,t
for f in ['K14C3_IMPLEMENTATION.md','K14C3_TESTER_GUIDE.md']: assert (root/f).exists(),f
print('Titanweave K14.C3 Radeon bare-metal staging source checks passed.')
