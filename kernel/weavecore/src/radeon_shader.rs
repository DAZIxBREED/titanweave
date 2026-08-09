//! K14.C31 owned shader/resource upload model.
//!
//! C31 uses a compact Titanweave reference shader format for QEMU qualification.
//! The bytes are real driver-owned GTT objects with SHA-256 identities. They are
//! not mislabeled as native AMD ISA; native GFX12 ISA upload remains a physical
//! hardware path gated by the C31 authority model.

use core::ptr;
use crate::{memory::{FrameAllocator,FRAME_SIZE},radeon_memory::{self,RadeonMemoryObject},sha256};

pub const RADEON_SHADER_ABI_VERSION:u32=1;
pub const MAX_C31_SHADER_BYTES:u64=64*1024;
pub const TW_SHADER_MAGIC:u32=u32::from_le_bytes(*b"TWSH"); // on-wire bytes: T W S H

#[repr(u8)]#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum ShaderStage{Compute=1,Vertex=2,Pixel=3}
#[repr(u8)]#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum ReferenceShaderKind{VectorAddU32=1,TriangleVertex=2,SolidPixel=3}
#[derive(Clone,Copy,Debug)]
pub struct ShaderProgram{pub object_id:u64,pub gpu_address:u64,pub kernel_address:u64,pub byte_len:u32,pub stage:ShaderStage,pub kind:ReferenceShaderKind,pub digest:[u8;32],pub native_amd_isa:bool}
impl ShaderProgram{pub const EMPTY:Self=Self{object_id:0,gpu_address:0,kernel_address:0,byte_len:0,stage:ShaderStage::Compute,kind:ReferenceShaderKind::VectorAddU32,digest:[0;32],native_amd_isa:false};pub fn valid(&self)->bool{self.object_id!=0&&self.gpu_address!=0&&self.kernel_address!=0&&self.byte_len>=8&&self.digest!=[0;32]}}

pub const COMPUTE_VECTOR_ADD:[u8;12]=[b'T',b'W',b'S',b'H',1,1,1,0,4,0,0,0];
pub const VERTEX_TRIANGLE:[u8;12]=[b'T',b'W',b'S',b'H',1,2,2,0,3,0,0,0];
pub const PIXEL_SOLID:[u8;12]=[b'T',b'W',b'S',b'H',1,3,3,0,1,0,0,0];

fn validate_blob(stage:ShaderStage,kind:ReferenceShaderKind,bytes:&[u8])->Result<(),&'static str>{
 if bytes.len()<8||bytes.len() as u64>MAX_C31_SHADER_BYTES{return Err("C31 shader byte length invalid")}
 if u32::from_le_bytes([bytes[0],bytes[1],bytes[2],bytes[3]])!=TW_SHADER_MAGIC{return Err("C31 shader magic invalid")}
 if bytes[4]!=1||bytes[5]!=stage as u8||bytes[6]!=kind as u8{return Err("C31 shader header/stage mismatch")}
 Ok(())
}
pub fn upload(allocator:&mut FrameAllocator<'_>,owner:u64,stage:ShaderStage,kind:ReferenceShaderKind,bytes:&[u8])->Result<ShaderProgram,&'static str>{
 validate_blob(stage,kind,bytes)?;let o=radeon_memory::allocate_gtt(allocator,owner,bytes.len() as u64,FRAME_SIZE)?;
 if o.kernel_virtual==0||o.gpu_virtual==0{return Err("C31 shader upload backing invalid")}
 unsafe{ptr::copy_nonoverlapping(bytes.as_ptr(),o.kernel_virtual as *mut u8,bytes.len())};
 let rb=unsafe{core::slice::from_raw_parts(o.kernel_virtual as *const u8,bytes.len())};if rb!=bytes{return Err("C31 shader upload readback mismatch")}
 radeon_memory::pin(owner,o.id,true)?;let digest=sha256::digest(bytes);if digest==[0;32]{return Err("C31 shader digest invalid")}
 Ok(ShaderProgram{object_id:o.id,gpu_address:o.gpu_virtual,kernel_address:o.kernel_virtual,byte_len:bytes.len() as u32,stage,kind,digest,native_amd_isa:false})
}
pub fn release(allocator:&mut FrameAllocator<'_>,owner:u64,p:ShaderProgram)->Result<(),&'static str>{radeon_memory::pin(owner,p.object_id,false)?;radeon_memory::free(allocator,owner,p.object_id)}
pub fn object(owner:u64,p:ShaderProgram)->Option<RadeonMemoryObject>{radeon_memory::object(owner,p.object_id)}
