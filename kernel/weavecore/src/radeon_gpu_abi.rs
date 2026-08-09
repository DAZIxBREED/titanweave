//! K14.C32 frozen userspace Radeon status/capability ABI.

pub const RADEON_USER_GPU_ABI_VERSION:u32=1;
pub const RADEON_C32_STATUS_SYSCALL:u64=43;
pub const CAP_QUEUE_STRESS:u64=1<<1;
pub const CAP_MEMORY_PRESSURE:u64=1<<2;
pub const CAP_RECOVERY:u64=1<<3;
pub const CAP_CONCURRENCY:u64=1<<4;
pub const CAP_MULTI_DISPLAY:u64=1<<5;
pub const CAP_TELEMETRY:u64=1<<6;
pub const CAP_POWER_POLICY:u64=1<<7;
pub const CAP_MULTI_GPU_ENUM:u64=1<<8;
pub const CAP_SHADER_PRECACHE:u64=1<<9;
pub const CAP_HARDWARE_DEFERRED:u64=1<<10;
pub const CAP_PHYSICAL_STRESS_QUALIFIED:u64=1<<11;
pub const CAP_BARE_METAL_SUITE_READY:u64=1<<12;
pub const CAP_QUALIFIED:u64=1<<13;
#[derive(Clone,Copy,Debug)]pub struct UserGpuAbi{pub version:u32,pub syscall:u64,pub stable_mask:u64,pub capability_mask:u64,pub fingerprint:u64}
pub fn frozen()->Result<UserGpuAbi,&'static str>{if RADEON_USER_GPU_ABI_VERSION!=1||RADEON_C32_STATUS_SYSCALL!=43{return Err("C32 userspace GPU ABI version/syscall mismatch")}let stable=1|CAP_QUEUE_STRESS|CAP_MEMORY_PRESSURE|CAP_RECOVERY|CAP_CONCURRENCY|CAP_MULTI_DISPLAY|CAP_TELEMETRY|CAP_POWER_POLICY|CAP_MULTI_GPU_ENUM|CAP_SHADER_PRECACHE|CAP_HARDWARE_DEFERRED|CAP_PHYSICAL_STRESS_QUALIFIED|CAP_BARE_METAL_SUITE_READY|CAP_QUALIFIED;let fp=0xc032_4142_4946_0001u64^stable^RADEON_C32_STATUS_SYSCALL^(u64::from(RADEON_USER_GPU_ABI_VERSION)<<32);Ok(UserGpuAbi{version:RADEON_USER_GPU_ABI_VERSION,syscall:RADEON_C32_STATUS_SYSCALL,stable_mask:stable,capability_mask:stable,fingerprint:fp})}
