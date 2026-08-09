//! K14.C31 bounded shader cache and precache contract.

use crate::{radeon_shader::{ShaderProgram,ShaderStage},sync::SpinLock};
pub const RADEON_SHADER_CACHE_ABI_VERSION:u32=1;
pub const C31_SHADER_CACHE_ENTRIES:usize=16;
#[derive(Clone,Copy)]pub struct CacheEntry{pub valid:bool,pub digest:[u8;32],pub object_id:u64,pub stage:ShaderStage,pub hits:u64,pub precached:bool}
impl CacheEntry{pub const EMPTY:Self=Self{valid:false,digest:[0;32],object_id:0,stage:ShaderStage::Compute,hits:0,precached:false};}
pub struct ShaderCache{entries:[CacheEntry;C31_SHADER_CACHE_ENTRIES],insertions:u64,hits:u64,misses:u64,precache:u64}
impl ShaderCache{
 pub const fn new()->Self{Self{entries:[CacheEntry::EMPTY;C31_SHADER_CACHE_ENTRIES],insertions:0,hits:0,misses:0,precache:0}}
 pub fn insert(&mut self,p:ShaderProgram,precache:bool)->Result<(),&'static str>{if !p.valid(){return Err("C31 cache rejects invalid shader")};if let Some(i)=self.entries.iter().position(|e|e.valid&&e.digest==p.digest){self.entries[i].object_id=p.object_id;self.entries[i].stage=p.stage;self.entries[i].precached|=precache;if precache{self.precache+=1}return Ok(())}let i=self.entries.iter().position(|e|!e.valid).ok_or("C31 shader cache full")?;self.entries[i]=CacheEntry{valid:true,digest:p.digest,object_id:p.object_id,stage:p.stage,hits:0,precached:precache};self.insertions+=1;if precache{self.precache+=1}Ok(())}
 pub fn lookup(&mut self,digest:[u8;32])->Option<CacheEntry>{if let Some(i)=self.entries.iter().position(|e|e.valid&&e.digest==digest){self.entries[i].hits+=1;self.hits+=1;Some(self.entries[i])}else{self.misses+=1;None}}
 pub fn counters(&self)->(u64,u64,u64,u64){(self.insertions,self.hits,self.misses,self.precache)}
}
static CACHE:SpinLock<ShaderCache>=SpinLock::new(ShaderCache::new());
pub fn insert(p:ShaderProgram,precache:bool)->Result<(),&'static str>{CACHE.lock().insert(p,precache)}
pub fn lookup(d:[u8;32])->Option<CacheEntry>{CACHE.lock().lookup(d)}
pub fn counters()->(u64,u64,u64,u64){CACHE.lock().counters()}
pub fn self_test()->Result<u64,&'static str>{let mut c=ShaderCache::new();let p=ShaderProgram{object_id:7,gpu_address:0x1000,kernel_address:0x2000,byte_len:12,stage:ShaderStage::Compute,kind:crate::radeon_shader::ReferenceShaderKind::VectorAddU32,digest:[0x5a;32],native_amd_isa:false};c.insert(p,true)?;if c.lookup(p.digest).map(|e|e.object_id)!=Some(7){return Err("C31 shader cache hit failed")};if c.lookup([0x33;32]).is_some(){return Err("C31 shader cache miss failed")};let (i,h,m,pc)=c.counters();if (i,h,m,pc)!=(1,1,1,1){return Err("C31 shader cache counters failed")}Ok(0xc031_5343_4143_0001^i^(h<<8)^(m<<16)^(pc<<24))}
