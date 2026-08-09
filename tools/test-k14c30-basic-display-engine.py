#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
paths=[
 root/'kernel/weavecore/src/radeon_edid.rs',root/'kernel/weavecore/src/radeon_dcn401.rs',
 root/'kernel/weavecore/src/radeon_display.rs',root/'kernel/weavecore/src/native_gpu_c30.rs',
 root/'kernel/weavecore/src/framebuffer.rs']
files={p.name:p.read_text() for p in paths};joined='\n'.join(files.values())
for bad in ['todo!()', 'unimplemented!()', 'todo!("', 'unimplemented!("']:
    assert bad not in joined, f'C30 contains forbidden stub primitive: {bad}'
e=files['radeon_edid.rs']
for token in ['EDID_BASE_BYTES: usize = 128','EDID checksum invalid','detailed timing','choose_mode','2560x1440','info.manufacturer!=*b"TWN"']:
    assert token in e,token
d=files['radeon_dcn401.rs']
for token in ['DCN401_TIMING_GENERATORS:u8=4','DCN401_VIDEO_PLANES:u8=4','DCN401_STREAM_ENCODERS:u8=4','DCN401_DDC:u8=4','DCN401_DSC:u8=4','DCN401_I2C_KHZ:u32=95','source_reviewed:true']:
    assert token in d,token
r=files['radeon_display.rs']
for token in ['MAX_DISPLAY_CONNECTORS:usize=4','HotplugJournal','allocate_gtt','copy_xrgb8888','sample_hash','front_object','back_object','flips:2','atomic_commit','GOP backend cannot change firmware mode after ExitBootServices','native_dcn_programmed:false','physical_hpd_enabled:false']:
    assert token in r,token
f=files['framebuffer.rs']
for token in ['copy_xrgb8888','write_volatile','sample_hash','read_volatile']:
    assert token in f,token
n=files['native_gpu_c30.rs']
for token in ['K14C30_ABI_VERSION:u32=1','RADEON_C30_NATIVE_DCN_MMIO_WRITES:bool=false','RADEON_C30_PLACEHOLDER_SUBSYSTEMS:u8=0','[C30ED]','[C30CN]','[C30SC]','[C30MS]','[C30HP]','[C30DC]','[C30PG]','[C30RD]']:
    assert token in n,token
main=(root/'kernel/weavecore/src/main.rs').read_text();proc=(root/'kernel/weavecore/src/process.rs').read_text();abi=(root/'kernel/weavecore/src/abi.rs').read_text();sys=(root/'kernel/weavecore/src/syscalls.rs').read_text();tw=(root/'userspace/include/twabi.inc').read_text();disp=(root/'userspace/displayd/displayd.S').read_text()
for token in ['mod native_gpu_c30;','mod radeon_edid;','mod radeon_dcn401;','mod radeon_display;','[C30OK]']:
    assert token in main,token
assert 'SYS_NATIVE_GPU_C30_QUERY: u64 = 41' in abi
assert 'SYS_NATIVE_GPU_C30_QUERY =>' in sys and 'native_gpu_c30::packed_status' in sys
assert '.equ TW_SYS_NATIVE_GPU_C30_QUERY, 41' in tw
for token in ['K14.C30 complete basic display engine online','QEMU/GOP backend verified','native DCN programming is not falsely claimed']:
    assert token in disp,token
for token in ['[KERN] K14.C30 alive:','[QUAL] K14.C30 complete-basic-display-engine runtime reached intentional post-userspace halt','[HALT] BSP halted intentionally']:
    assert token in proc,token
runner=(root/'tools/run-k14c30-qemu-basic-display-engine.sh').read_text();checker=(root/'tools/check-k14c30-serial-log.sh').read_text()
for token in ['Intentional Titanweave HALT detected','check-k14c30-serial-log.sh','-vga std']:
    assert token in runner,token
for token in ['[C30ED] EDID/mode engine:','[C30SC] double-buffer scanout:','Titanweave K14.C30 complete-basic-display-engine runtime qualification PASSED.']:
    assert token in checker,token
print('Titanweave K14.C30 operational basic display engine source checks passed.')
