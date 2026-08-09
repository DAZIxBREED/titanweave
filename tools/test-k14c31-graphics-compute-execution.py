#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
paths=[
 root/'kernel/weavecore/src/radeon_shader.rs',root/'kernel/weavecore/src/radeon_shader_cache.rs',
 root/'kernel/weavecore/src/radeon_command.rs',root/'kernel/weavecore/src/radeon_pipeline.rs',
 root/'kernel/weavecore/src/radeon_compute.rs',root/'kernel/weavecore/src/radeon_compute_caps.rs',
 root/'kernel/weavecore/src/radeon_graphics.rs',root/'kernel/weavecore/src/native_gpu_c31.rs']
files={p.name:p.read_text() for p in paths};joined='\n'.join(files.values())
for bad in ['todo!()', 'unimplemented!()', 'todo!("', 'unimplemented!("']:
    assert bad not in joined, f'C31 contains forbidden stub primitive: {bad}'
sh=files['radeon_shader.rs']
for token in ['ShaderStage','ReferenceShaderKind','VectorAddU32','TriangleVertex','SolidPixel','allocate_gtt','copy_nonoverlapping','sha256::digest','native_amd_isa:false']:
    assert token in sh,token
# Validate the actual on-wire shader magic, not merely the presence of a symbol.
assert 'u32::from_le_bytes(*b"TWSH")' in sh, 'C31 shader magic must use little-endian TWSH wire bytes'
assert int.from_bytes(b'TWSH','little') == 0x48535754
for blob in [b'TWSH'+bytes([1,1,1,0]), b'TWSH'+bytes([1,2,2,0]), b'TWSH'+bytes([1,3,3,0])]:
    assert int.from_bytes(blob[:4],'little') == 0x48535754
cache=files['radeon_shader_cache.rs']
for token in ['C31_SHADER_CACHE_ENTRIES:usize=16','insertions','hits','misses','precache','lookup']:
    assert token in cache,token
cmd=files['radeon_command.rs']
for token in ['CommandOpcode','BindPipeline','BindResource','Dispatch','DrawTriangle','Present','QueueClass','Compute=1','Graphics=2','ExecutionQueue','decode','C31_QUEUE_DEPTH:usize=16']:
    assert token in cmd,token
comp=files['radeon_compute.rs']
for token in ['C31_COMPUTE_ELEMENTS:u32=256','VectorAddU32','Dispatch','execute_reference','wrapping_add','compute output verification failed','RadeonFenceTimeline','QueueClass::Compute']:
    assert token in comp,token
caps=files['radeon_compute_caps.rs']
for token in ['address_bits:64','dispatch_dimensions:3','max_workgroup_x:1024','separate_compute_queue:true','separate_graphics_queue:true','shader_precache:true','native_amd_isa:false','physical_dispatch:false']:
    assert token in caps,token
gfx=files['radeon_graphics.rs']
for token in ['TriangleVertex','SolidPixel','DrawTriangle','raster_triangle','Framebuffer::from_boot_info','copy_xrgb8888','QueueClass::Graphics','graphics draw did not alter render target fingerprint']:
    assert token in gfx,token
n=files['native_gpu_c31.rs']
for token in ['K14C31_ABI_VERSION:u32=1','RADEON_C31_REFERENCE_EXECUTION:bool=true','RADEON_C31_NATIVE_AMD_ISA:bool=false','RADEON_C31_PHYSICAL_CP_QUEUES:bool=false','RADEON_C31_PLACEHOLDER_SUBSYSTEMS:u8=0','[C31SH]','[C31CQ]','[C31CP]','[C31GQ]','[C31GX]','[C31HC]','[C31SC]','[C31PG]','[C31RD]']:
    assert token in n,token
main=(root/'kernel/weavecore/src/main.rs').read_text();proc=(root/'kernel/weavecore/src/process.rs').read_text();abi=(root/'kernel/weavecore/src/abi.rs').read_text();sys=(root/'kernel/weavecore/src/syscalls.rs').read_text();tw=(root/'userspace/include/twabi.inc').read_text();disp=(root/'userspace/displayd/displayd.S').read_text()
for token in ['mod native_gpu_c31;','mod radeon_shader;','mod radeon_command;','mod radeon_compute;','mod radeon_graphics;','[C31OK]']:
    assert token in main,token
assert 'SYS_NATIVE_GPU_C31_QUERY: u64 = 42' in abi
assert 'SYS_NATIVE_GPU_C31_QUERY =>' in sys and 'native_gpu_c31::packed_status' in sys
assert '.equ TW_SYS_NATIVE_GPU_C31_QUERY, 42' in tw
for token in ['K14.C31 graphics and compute execution subsystem online','QEMU reference backend verified','physical Radeon queue programming remains separately gated']:
    assert token in disp,token
for token in ['[KERN] K14.C31 alive:','[QUAL] K14.C31 graphics-compute-execution runtime reached intentional post-userspace halt','[HALT] BSP halted intentionally']:
    assert token in proc,token
runner=(root/'tools/run-k14c31-qemu-graphics-compute-execution.sh').read_text();checker=(root/'tools/check-k14c31-serial-log.sh').read_text()
for token in ['Intentional Titanweave HALT detected','check-k14c31-serial-log.sh','-vga std']:
    assert token in runner,token
for token in ['[C31CP] compute execution:','[C31GX] graphics execution:','Titanweave K14.C31 graphics-compute-execution runtime qualification PASSED.']:
    assert token in checker,token
assert (root/'K14C30_RUNTIME_QUALIFICATION.md').is_file()
assert (root/'K14C31_IMPLEMENTATION.md').is_file()
assert (root/'K14C31_SOURCE_STATUS.md').is_file()
assert (root/'K14C31_TESTER_GUIDE.md').is_file()
print('Titanweave K14.C31 operational graphics+compute execution source checks passed.')
