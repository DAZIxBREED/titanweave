#[derive(Clone,Copy,PartialEq,Eq)] pub enum AccessMode{None,ReadOnly,ReadWrite}
#[derive(Clone,Copy)] pub struct MountGrant{pub volume_id:[u8;16],pub mode:AccessMode}
pub struct MountNamespace{grants:[Option<MountGrant>;16]}
impl MountNamespace{pub const fn empty()->Self{Self{grants:[None;16]}} pub fn grant(&mut self,g:MountGrant)->Result<(),&'static str>{for slot in &mut self.grants{if slot.is_none(){*slot=Some(g);return Ok(())}}Err("mount namespace grant table is full")} pub fn access(&self,id:[u8;16])->AccessMode{for g in self.grants.iter().flatten(){if g.volume_id==id{return g.mode}}AccessMode::None}}
