//! K14.C31 bounded compute/graphics pipeline state.
use crate::radeon_shader::{ShaderProgram,ShaderStage};
pub const RADEON_PIPELINE_ABI_VERSION:u32=1;
#[derive(Clone,Copy,Debug)]pub struct ComputePipeline{pub id:u64,pub shader:ShaderProgram,pub local_size_x:u32,pub valid:bool}
#[derive(Clone,Copy,Debug)]pub struct GraphicsPipeline{pub id:u64,pub vertex:ShaderProgram,pub pixel:ShaderProgram,pub render_target:u64,pub width:u32,pub height:u32,pub stride:u32,pub valid:bool}
pub fn compute(id:u64,shader:ShaderProgram,local_size_x:u32)->Result<ComputePipeline,&'static str>{if id==0||!shader.valid()||shader.stage!=ShaderStage::Compute||local_size_x==0||local_size_x>1024{return Err("C31 compute pipeline invalid")};Ok(ComputePipeline{id,shader,local_size_x,valid:true})}
pub fn graphics(id:u64,vertex:ShaderProgram,pixel:ShaderProgram,render_target:u64,width:u32,height:u32,stride:u32)->Result<GraphicsPipeline,&'static str>{if id==0||!vertex.valid()||!pixel.valid()||vertex.stage!=ShaderStage::Vertex||pixel.stage!=ShaderStage::Pixel||render_target==0||width<64||height<64||stride<width.saturating_mul(4){return Err("C31 graphics pipeline invalid")};Ok(GraphicsPipeline{id,vertex,pixel,render_target,width,height,stride,valid:true})}
