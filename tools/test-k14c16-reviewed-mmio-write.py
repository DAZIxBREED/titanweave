from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
c=text('kernel/weavecore/src/native_gpu_c16.rs'); main=text('kernel/weavecore/src/main.rs')
for x in ['mod native_gpu_c16;','native_gpu_c16::initialize(&mut allocator, boot_info.bootstrap.page_table_root)','[C16OK] K14.C16 reviewed Radeon MMIO-write gate:']: assert x in main,x
for x in ['K14C16_ABI_VERSION','Gfx12GcScratchReg0','[C16RV]','[C16PG]','[C16HW]','[C16RD]','map_kernel_mmio','write_volatile','RADEON_C16_MAX_READBACK_POLLS']: assert x in c,x
for x in ['RADEON_C16_FIRMWARE_UPLOAD_ALLOWED:bool=false','RADEON_C16_COMMAND_SUBMIT_ALLOWED:bool=false','RADEON_C16_BUS_MASTER_ALLOWED:bool=false','target_resolved=false','guessed_offsets=false']: assert x in c,x
assert 'SYS_NATIVE_GPU_C16_QUERY: u64 = 27' in text('kernel/weavecore/src/abi.rs')
assert 'TW_SYS_NATIVE_GPU_C16_QUERY, 27' in text('userspace/include/twabi.inc')
s=text('kernel/weavecore/src/syscalls.rs'); assert 'SYS_NATIVE_GPU_C16_QUERY' in s and 'native_gpu_c16::packed_status()' in s
d=text('userspace/displayd/displayd.S')
for x in ['K14.C16 reviewed Radeon MMIO-write transaction gate online','no physical Radeon in QEMU','exact reviewed MMIO target/base is unresolved']: assert x in d,x
pr=text('kernel/weavecore/src/process.rs')
for x in ['[KERN] K14.C16 alive:','[QUAL] K14.C16 reviewed-MMIO-write runtime reached intentional post-userspace halt']: assert x in pr,x
for f in ['K14C16_IMPLEMENTATION.md','K14C16_TESTER_GUIDE.md']: assert (root/'docs'/f).exists(),f
print('Titanweave K14.C16 reviewed MMIO-write source checks passed.')
