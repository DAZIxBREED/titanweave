//! K14.C31 compute capability contract for future HIP/ROCm-style runtimes.
//!
//! This is a Titanweave kernel capability surface, not an implementation of the
//! HIP or ROCm user APIs. It freezes the properties C31 can actually provide to
//! later runtimes while keeping native ISA/silicon dispatch false.
pub const RADEON_COMPUTE_CAPS_ABI_VERSION:u32=1;
#[derive(Clone,Copy,Debug)]pub struct ComputeCapabilities{pub address_bits:u8,pub dispatch_dimensions:u8,pub max_workgroup_x:u16,pub separate_compute_queue:bool,pub separate_graphics_queue:bool,pub timeline_fence:bool,pub host_visible_gtt:bool,pub shader_cache:bool,pub shader_precache:bool,pub native_amd_isa:bool,pub physical_dispatch:bool,pub source_stable:bool,pub fingerprint:u64}
pub fn current()->Result<ComputeCapabilities,&'static str>{let mut c=ComputeCapabilities{address_bits:64,dispatch_dimensions:3,max_workgroup_x:1024,separate_compute_queue:true,separate_graphics_queue:true,timeline_fence:true,host_visible_gtt:true,shader_cache:true,shader_precache:true,native_amd_isa:false,physical_dispatch:false,source_stable:true,fingerprint:0};if c.address_bits!=64||c.dispatch_dimensions!=3||c.max_workgroup_x!=1024||!c.separate_compute_queue||!c.separate_graphics_queue||!c.timeline_fence||!c.host_visible_gtt||!c.shader_cache||!c.shader_precache||c.native_amd_isa||c.physical_dispatch{return Err("C31 compute capability model invalid")};c.fingerprint=0xc031_4849_5043_0001u64^u64::from(c.max_workgroup_x)^(u64::from(c.address_bits)<<16)^(u64::from(c.dispatch_dimensions)<<24);Ok(c)}
