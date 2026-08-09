#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p):return(root/p).read_text()
main=text('kernel/weavecore/src/main.rs')
for t in ['mod native_gpu_c6;','native_gpu_c6::initialize(&mut allocator, boot_info.bootstrap.page_table_root)','[C6NF] K14.C6 live AMD-Vi engine ready:']:assert t in main,t
c6=text('kernel/weavecore/src/native_gpu_c6.rs')
for t in ['K14C6_ABI_VERSION','AmdViHardwarePlan','AMDVI_C6_ENABLE_BARE_METAL_PROGRAMMING','hardware_programming_eligible','persistent_domain_live','[C6RG]','[C6SQ]','[C6HW]','[C6RD]']:assert t in c6,t
amd=text('kernel/weavecore/src/amd_vi.rs')
for t in ['AMDVI_REG_DEVICE_TABLE_BASE','AMDVI_REG_COMMAND_BUFFER_BASE','AMDVI_REG_EVENT_LOG_BASE','AMDVI_REG_CONTROL','AMDVI_REG_STATUS','program_hardware_unit','c6_register_self_test']:assert t in amd,t
assert 'SYS_NATIVE_GPU_C6_QUERY: u64 = 17' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C6_QUERY, 17' in text('userspace/include/twabi.inc')
sys=text('kernel/weavecore/src/syscalls.rs');assert 'SYS_NATIVE_GPU_C6_QUERY' in sys and 'native_gpu_c6::packed_status()' in sys
d=text('userspace/displayd/displayd.S')
for t in ['K14.C6 live AMD-Vi hardware-programming boundary online','no physical Radeon in QEMU','bare-metal activation gate remains unarmed']:assert t in d,t
p=text('kernel/weavecore/src/process.rs')
for t in ['[KERN] K14.C6 alive:','[QUAL] K14.C6 live AMD-Vi runtime reached intentional post-userspace halt']:assert t in p,t
for f in ['K14C6_IMPLEMENTATION.md','K14C6_TESTER_GUIDE.md']:assert(root/f).exists(),f
print('Titanweave K14.C6 live AMD-Vi source checks passed.')
