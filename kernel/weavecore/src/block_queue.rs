//! K11.0 bounded asynchronous block-request lifecycle.
use crate::block::SECTOR_SIZE;

pub const MAX_BLOCK_REQUESTS: usize = 128;
pub const MAX_BLOCK_SEGMENTS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockOperation { Read, Write, Flush, Discard }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockRequestState { Free, Queued, Active, CancelPending, TimeoutPending, Completed, Failed, Cancelled, TimedOut }

#[derive(Clone, Copy, Debug)]
pub struct BlockSegment { pub physical: u64, pub bytes: u32 }
impl BlockSegment { pub const EMPTY: Self = Self { physical: 0, bytes: 0 }; }

#[derive(Clone, Copy, Debug)]
pub struct BlockRequest { pub id:u64,pub device_id:u64,pub operation:BlockOperation,pub first_lba:u64,pub sector_count:u32,pub deadline_tick:u64,pub owner:u64,pub state:BlockRequestState,pub result:i32,pub segments:[BlockSegment;MAX_BLOCK_SEGMENTS],pub segment_count:usize }
impl BlockRequest {
 pub const EMPTY:Self=Self{id:0,device_id:0,operation:BlockOperation::Read,first_lba:0,sector_count:0,deadline_tick:0,owner:0,state:BlockRequestState::Free,result:0,segments:[BlockSegment::EMPTY;MAX_BLOCK_SEGMENTS],segment_count:0};
 pub fn validate(&self)->Result<(), &'static str>{
  if self.device_id==0||self.owner==0{return Err("block request has no device or owner")}
  if self.segment_count>MAX_BLOCK_SEGMENTS{return Err("too many block segments")}
  match self.operation {
   BlockOperation::Read|BlockOperation::Write=>{if self.sector_count==0||self.segment_count==0{return Err("data request is empty")}let expected=(self.sector_count as u64).checked_mul(SECTOR_SIZE as u64).ok_or("block length overflow")?;let mut actual=0u64;for s in &self.segments[..self.segment_count]{if s.physical==0||s.bytes==0{return Err("invalid block segment")}actual=actual.checked_add(s.bytes as u64).ok_or("segment length overflow")?;}if actual!=expected{return Err("scatter/gather length does not match sector count")}}
   BlockOperation::Flush=>if self.sector_count!=0||self.segment_count!=0{return Err("flush request carries data")},
   BlockOperation::Discard=>if self.sector_count==0||self.segment_count!=0{return Err("discard request is malformed")},
  } Ok(())
 }
}

pub struct BlockRequestQueue{slots:[BlockRequest;MAX_BLOCK_REQUESTS],next_id:u64}
impl BlockRequestQueue{
 pub const fn new()->Self{Self{slots:[BlockRequest::EMPTY;MAX_BLOCK_REQUESTS],next_id:1}}
 pub fn submit(&mut self,mut r:BlockRequest)->Result<u64,&'static str>{r.validate()?;let s=self.slots.iter_mut().find(|s|matches!(s.state,BlockRequestState::Free|BlockRequestState::Completed|BlockRequestState::Failed|BlockRequestState::Cancelled|BlockRequestState::TimedOut)).ok_or("block request queue full")?;r.id=self.next_id;self.next_id=self.next_id.checked_add(1).ok_or("block request id exhausted")?;r.state=BlockRequestState::Queued;*s=r;Ok(r.id)}
 pub fn claim(&mut self,device_id:u64)->Option<BlockRequest>{let s=self.slots.iter_mut().find(|r|r.state==BlockRequestState::Queued&&r.device_id==device_id)?;s.state=BlockRequestState::Active;Some(*s)}
 pub fn complete(&mut self,id:u64,result:i32)->Result<(),&'static str>{let r=self.slots.iter_mut().find(|r|r.id==id).ok_or("block request not found")?;if r.state!=BlockRequestState::Active{return Err("block request is not active")};r.result=result;r.state=if result==0{BlockRequestState::Completed}else{BlockRequestState::Failed};Ok(())}
 /// Queued work can be cancelled immediately. Active DMA becomes pending and
 /// remains pinned until the device backend confirms reset/fence completion.
 pub fn request_cancel_owner(&mut self,owner:u64)->usize{let mut n=0;for r in &mut self.slots{if r.owner!=owner{continue}match r.state{BlockRequestState::Queued=>{r.state=BlockRequestState::Cancelled;n+=1},BlockRequestState::Active=>{r.state=BlockRequestState::CancelPending;n+=1},_=>{}}}n}
 pub fn expire(&mut self,now:u64)->usize{let mut n=0;for r in &mut self.slots{if r.deadline_tick==0||now<r.deadline_tick{continue}match r.state{BlockRequestState::Queued=>{r.state=BlockRequestState::TimedOut;n+=1},BlockRequestState::Active=>{r.state=BlockRequestState::TimeoutPending;n+=1},_=>{}}}n}
 /// Called only after the controller has stopped bus mastering, completed an
 /// abort, or its IOMMU mappings have been revoked and invalidated.
 pub fn fence_device(&mut self,device_id:u64,result:i32)->usize{let mut n=0;for r in &mut self.slots{if r.device_id!=device_id{continue}match r.state{BlockRequestState::CancelPending=>{r.result=result;r.state=BlockRequestState::Cancelled;n+=1},BlockRequestState::TimeoutPending=>{r.result=result;r.state=BlockRequestState::TimedOut;n+=1},_=>{}}}n}
 pub fn owner_has_unfenced(&self,owner:u64)->bool{self.slots.iter().any(|r|r.owner==owner&&matches!(r.state,BlockRequestState::Active|BlockRequestState::CancelPending|BlockRequestState::TimeoutPending))}
 pub fn devices_needing_fence(&self,owner:u64,mut f:impl FnMut(u64)){let mut seen=[0u64;MAX_BLOCK_REQUESTS];let mut count=0;for r in &self.slots{if r.owner!=owner||!matches!(r.state,BlockRequestState::CancelPending|BlockRequestState::TimeoutPending){continue}if !seen[..count].contains(&r.device_id){seen[count]=r.device_id;count+=1;f(r.device_id)}}}
 pub fn status(&self,id:u64)->Option<(BlockRequestState,i32)>{self.slots.iter().find(|r|r.id==id).map(|r|(r.state,r.result))}
}
