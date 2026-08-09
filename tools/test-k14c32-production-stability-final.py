#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
paths=[
 root/'kernel/weavecore/src/radeon_command.rs',root/'kernel/weavecore/src/radeon_stability.rs',
 root/'kernel/weavecore/src/radeon_telemetry.rs',root/'kernel/weavecore/src/radeon_power.rs',
 root/'kernel/weavecore/src/radeon_multigpu.rs',root/'kernel/weavecore/src/radeon_gpu_abi.rs',
 root/'kernel/weavecore/src/native_gpu_c32.rs']
files={p.name:p.read_text() for p in paths};joined='\n'.join(files.values())
for bad in ['todo!()', 'unimplemented!()', 'TODO', 'PLACEHOLDER IMPLEMENTATION', 'fake_success']:
    assert bad not in joined, f'C32 contains forbidden stub/placeholder marker: {bad}'
cmd=files['radeon_command.rs']
for token in ['pub fn reset(&mut self)->usize','stability_counters','abandoned','resets']:
    assert token in cmd,token
st=files['radeon_stability.rs']
for token in ['C32_QUEUE_STRESS_ROUNDS:u32=32','C32_GTT_PRESSURE_OBJECTS:usize=12','C32_VRAM_PRESSURE_OBJECTS:usize=4','C32_IRQ_STRESS_EVENTS:u32=1024','C32_RECOVERY_STRESS_CYCLES:u32=32','C32_CONCURRENCY_ROUNDS:u32=64','C32_DISPLAY_STRESS_PRESENTS:u32=16','queue_stress','hang_recovery','pressure_test','software_irq_stress','recovery_stress','concurrency_test','display_stress','multi_display_framework_test','Framebuffer::from_boot_info','copy_xrgb8888','allocate_gtt','reserve_vram','RadeonFenceTimeline','QueueClass::Compute','QueueClass::Graphics']:
    assert token in st,token
assert 'radeon_display::MAX_DISPLAY_CONNECTORS<4' in st, 'C32 multi-display gate must use the imported radeon_display module name'
assert 'if display::MAX_DISPLAY_CONNECTORS<4' not in st, 'C32 stale unresolved display module alias regressed'
tele=files['radeon_telemetry.rs']
for token in ['C32_TELEMETRY_EVENTS:usize=32','TelemetryEvent','TelemetrySnapshot','HangDetected','HangRecovered','compute_operations','graphics_pixels','bytes_touched','RADEON_C32_PHYSICAL_PERF_COUNTER_MMIO:bool=false']:
    assert token in tele,token
power=files['radeon_power.rs']
for token in ['PowerState','Boot=0','Active=1','Idle=2','Quiesced=3','Fault=4','RADEON_C32_PHYSICAL_SMU_PROGRAMMING:bool=false','illegal Radeon power transition']:
    assert token in power,token
mg=files['radeon_multigpu.rs']
for token in ['MAX_C32_GPU_ADAPTERS:usize=8','pci::enumerate','class_code!=0x03','AMD_VENDOR_ID:u16=0x1002','RADEON_C32_PEER_DMA_ENABLED:bool=false','RADEON_C32_CROSS_GPU_EXECUTION_ENABLED:bool=false']:
    assert token in mg,token
abi=files['radeon_gpu_abi.rs']
for token in ['RADEON_USER_GPU_ABI_VERSION:u32=1','RADEON_C32_STATUS_SYSCALL:u64=43','CAP_QUEUE_STRESS','CAP_MEMORY_PRESSURE','CAP_RECOVERY','CAP_CONCURRENCY','CAP_MULTI_DISPLAY','CAP_TELEMETRY','CAP_POWER_POLICY','CAP_MULTI_GPU_ENUM','CAP_SHADER_PRECACHE','CAP_PHYSICAL_STRESS_QUALIFIED','CAP_BARE_METAL_SUITE_READY','CAP_QUALIFIED']:
    assert token in abi,token
n=files['native_gpu_c32.rs']
for token in ['K14C32_ABI_VERSION:u32=1','RADEON_C32_PRODUCTION_STABILITY:bool=true','RADEON_C32_USER_ABI_FROZEN:bool=true','RADEON_C32_CAPABILITY_MODEL_FROZEN:bool=true','RADEON_C32_BARE_METAL_SUITE_READY:bool=true','RADEON_C32_PHYSICAL_STRESS_QUALIFIED_BY_QEMU:bool=false','RADEON_C32_PLACEHOLDER_SUBSYSTEMS:u8=0','[C32QS]','[C32MP]','[C32RC]','[C32CX]','[C32MD]','[C32PM]','[C32TL]','[C32AB]','[C32PG]','[C32RD]']:
    assert token in n,token
assert 'else{0xc030}' in n, 'C32 QEMU owner must inherit C30/C31 fallback ownership'
assert 'c31.amd_present&&c31.physical_gpu_execution&&stability.vram_pressure_verified' in n, 'physical stress may only derive from physical execution evidence'
main=(root/'kernel/weavecore/src/main.rs').read_text();proc=(root/'kernel/weavecore/src/process.rs').read_text();kabi=(root/'kernel/weavecore/src/abi.rs').read_text();sys=(root/'kernel/weavecore/src/syscalls.rs').read_text();tw=(root/'userspace/include/twabi.inc').read_text();disp=(root/'userspace/displayd/displayd.S').read_text();drv=(root/'kernel/weavecore/src/radeon_driver.rs').read_text()
for token in ['mod radeon_telemetry;','mod radeon_power;','mod radeon_multigpu;','mod radeon_gpu_abi;','mod radeon_stability;','mod native_gpu_c32;','[C32OK]']:
    assert token in main,token
assert 'SYS_NATIVE_GPU_C32_QUERY: u64 = 43' in kabi
assert 'SYS_NATIVE_GPU_C32_QUERY =>' in sys and 'native_gpu_c32::packed_status' in sys
assert '.equ TW_SYS_NATIVE_GPU_C32_QUERY, 43' in tw
assert 'pub fn software_irq_stress(rounds:u32)' in drv
for token in ['K14.C32 production/stability and final Radeon foundation online','K14.C32 QEMU production gate verified','K14 remains open and ForgeAudio must not begin']:
    assert token in disp,token
c32_qemu_user_msg='[displayd] K14.C32 QEMU production gate verified: queue/memory stress, recovery+IRQ, graphics+compute/display+compute coexistence, scanout, telemetry, power, frozen GPU ABI, shader precache, multi-GPU inventory; physical Radeon stress separate'
assert c32_qemu_user_msg in disp, 'C32 QEMU userspace qualification message changed unexpectedly'
assert len(c32_qemu_user_msg) <= 256, 'C32 QEMU userspace qualification message exceeds SYS_WRITE MAX_MESSAGE_BYTES'
for token in ['[KERN] K14.C32 alive:','[QUAL] K14.C32 production-stability-final runtime reached intentional post-userspace halt','[K14DONE] Titanweave native Radeon driver foundation operational','[K15NEXT] K15 ForgeAudio is the next locked Titanweave milestone','[HALT] BSP halted intentionally']:
    assert token in proc,token
# K14DONE must be post-userspace process output, not an early main.rs success print.
assert '[K14DONE]' not in main
runner=(root/'tools/run-k14c32-qemu-production-stability-final.sh').read_text();checker=(root/'tools/check-k14c32-serial-log.sh').read_text();bare=(root/'tools/check-k14c32-baremetal-log.sh').read_text()
assert c32_qemu_user_msg in checker, 'C32 serial checker must match emitted userspace qualification message exactly'
for token in ['Intentional Titanweave HALT detected','check-k14c32-serial-log.sh','-vga std','K14.C32']:
    assert token in runner,token
for token in ['[C32QS] queue stability:','[C32MP] memory pressure:','[C32CX] concurrency:','[C32TL] telemetry/diagnostics:','[C32AB] frozen GPU ABI/capabilities:','[K14DONE] Titanweave native Radeon driver foundation operational','Titanweave K14.C32 production-stability-final runtime qualification PASSED.']:
    assert token in checker,token
for token in ['physical_stress=true','amd_present=true','BARE-METAL']:
    assert token in bare,token
for f in ['K14C31_RUNTIME_QUALIFICATION.md','K14C32_IMPLEMENTATION.md','K14C32_TESTER_GUIDE.md','K14C32_SOURCE_STATUS.md','K14_LOCKED_ROADMAP.md']:
    assert (root/f).is_file(),f
print('Titanweave K14.C32 operational production/stability final source checks passed.')
