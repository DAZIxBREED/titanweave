//! K11.0 reference-counted kernel object lifecycle registry.
pub const MAX_LIVE_OBJECTS: usize = 512;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleState { Free, Live, Closing, Dead }
#[derive(Clone, Copy, Debug)]
pub struct LifecycleRecord { pub object_id:u64, pub references:u32, pub state:LifecycleState, pub owner:u64, pub generation:u32 }
impl LifecycleRecord { pub const EMPTY:Self=Self{object_id:0,references:0,state:LifecycleState::Free,owner:0,generation:0}; }
pub struct ObjectLifecycle { records:[LifecycleRecord;MAX_LIVE_OBJECTS] }
impl ObjectLifecycle {
    pub const fn new()->Self{Self{records:[LifecycleRecord::EMPTY;MAX_LIVE_OBJECTS]}}
    pub fn create(&mut self, object_id:u64, owner:u64)->Result<u32,&'static str>{
        if object_id==0{return Err("invalid object id")} if self.records.iter().any(|r|r.object_id==object_id&&r.state!=LifecycleState::Free){return Err("object already exists")}
        let slot=self.records.iter_mut().find(|r|matches!(r.state,LifecycleState::Free|LifecycleState::Dead)).ok_or("object registry full")?;
        let generation=slot.generation.wrapping_add(1).max(1);*slot=LifecycleRecord{object_id,references:1,state:LifecycleState::Live,owner,generation};Ok(generation)
    }
    pub fn retain(&mut self, object_id:u64, generation:u32)->Result<u32,&'static str>{let r=self.find_mut(object_id,generation)?;if r.state!=LifecycleState::Live{return Err("object is closing")}r.references=r.references.checked_add(1).ok_or("object reference overflow")?;Ok(r.references)}
    pub fn begin_close(&mut self,object_id:u64,generation:u32)->Result<(),&'static str>{let r=self.find_mut(object_id,generation)?;if r.state!=LifecycleState::Live{return Err("object is not live")}r.state=LifecycleState::Closing;Ok(())}
    pub fn release(&mut self,object_id:u64,generation:u32)->Result<bool,&'static str>{let r=self.find_mut(object_id,generation)?;if r.references==0{return Err("object reference underflow")}r.references-=1;if r.references==0{r.state=LifecycleState::Dead;return Ok(true)}Ok(false)}
    pub fn release_owner(&mut self,owner:u64)->usize{let mut n=0;for r in &mut self.records{if r.owner==owner&&matches!(r.state,LifecycleState::Live|LifecycleState::Closing){r.state=LifecycleState::Closing;if r.references>0{r.references-=1;}if r.references==0{r.state=LifecycleState::Dead;}n+=1}}n}
    pub fn release_object(&mut self,object_id:u64)->Result<bool,&'static str>{let r=self.records.iter_mut().find(|r|r.object_id==object_id&&!matches!(r.state,LifecycleState::Free|LifecycleState::Dead)).ok_or("object not found")?;if r.references==0{return Err("object reference underflow")}r.references-=1;if r.references==0{r.state=LifecycleState::Dead;Ok(true)}else{Ok(false)}}
    fn find_mut(&mut self,id:u64,g:u32)->Result<&mut LifecycleRecord,&'static str>{self.records.iter_mut().find(|r|r.object_id==id&&r.generation==g&&!matches!(r.state,LifecycleState::Free|LifecycleState::Dead)).ok_or("stale or missing object handle")}
}
