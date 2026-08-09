//! C29 bounded Radeon submission queue with explicit lifecycle and ordering.

use crate::sync::SpinLock;
pub const RADEON_QUEUE_ABI_VERSION:u32=1;
pub const C29_QUEUE_DEPTH:usize=32;
#[repr(u8)]#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum SubmissionStatus{Empty=0,Queued=1,Emitted=2,Retired=3,Cancelled=4}
#[derive(Clone,Copy,Debug)]pub struct Submission{pub id:u64,pub fence:u32,pub bytes:u32,pub status:SubmissionStatus}
impl Submission{pub const EMPTY:Self=Self{id:0,fence:0,bytes:0,status:SubmissionStatus::Empty};}
pub struct RadeonSubmissionQueue{entries:[Submission;C29_QUEUE_DEPTH],head:usize,tail:usize,count:usize,next_id:u64,submitted:u64,retired:u64,cancelled:u64}
impl RadeonSubmissionQueue{
 pub const fn new()->Self{Self{entries:[Submission::EMPTY;C29_QUEUE_DEPTH],head:0,tail:0,count:0,next_id:1,submitted:0,retired:0,cancelled:0}}
 pub fn enqueue(&mut self,bytes:u32,fence:u32)->Result<u64,&'static str>{if bytes==0||fence==0{return Err("C29 queue rejects empty submission")}if self.count==C29_QUEUE_DEPTH{return Err("C29 submission queue full")}let id=self.next_id;self.next_id=self.next_id.checked_add(1).ok_or("C29 queue id exhausted")?;self.entries[self.tail]=Submission{id,fence,bytes,status:SubmissionStatus::Queued};self.tail=(self.tail+1)%C29_QUEUE_DEPTH;self.count+=1;self.submitted+=1;Ok(id)}
 pub fn mark_emitted(&mut self,id:u64)->Result<(),&'static str>{let i=self.find(id)?;if self.entries[i].status!=SubmissionStatus::Queued{return Err("C29 queue emit transition invalid")}self.entries[i].status=SubmissionStatus::Emitted;Ok(())}
 pub fn retire_head(&mut self,completed_fence:u32)->Result<Option<Submission>,&'static str>{if self.count==0{return Ok(None)}let mut s=self.entries[self.head];if s.status!=SubmissionStatus::Emitted{return Ok(None)}if completed_fence<s.fence{return Ok(None)}s.status=SubmissionStatus::Retired;self.entries[self.head]=Submission::EMPTY;self.head=(self.head+1)%C29_QUEUE_DEPTH;self.count-=1;self.retired+=1;Ok(Some(s))}
 pub fn cancel_all(&mut self)->u64{let mut n=0;for e in &mut self.entries{if matches!(e.status,SubmissionStatus::Queued|SubmissionStatus::Emitted){e.status=SubmissionStatus::Cancelled;n+=1}*e=Submission::EMPTY}self.head=0;self.tail=0;self.count=0;self.cancelled+=n;n}
 fn find(&self,id:u64)->Result<usize,&'static str>{self.entries.iter().position(|e|e.id==id&&!matches!(e.status,SubmissionStatus::Empty|SubmissionStatus::Retired|SubmissionStatus::Cancelled)).ok_or("C29 queue submission not found")}
 pub fn counters(&self)->(u64,u64,u64,usize){(self.submitted,self.retired,self.cancelled,self.count)}
}
#[derive(Clone,Copy,Debug)]pub struct RadeonQueueState{pub ready:bool,pub ordering_verified:bool,pub cancellation_verified:bool,pub submitted:u64,pub retired:u64,pub cancelled:u64,pub fingerprint:u64}
impl RadeonQueueState{pub const EMPTY:Self=Self{ready:false,ordering_verified:false,cancellation_verified:false,submitted:0,retired:0,cancelled:0,fingerprint:0};}
static STATE:SpinLock<RadeonQueueState>=SpinLock::new(RadeonQueueState::EMPTY);
pub fn self_test()->Result<RadeonQueueState,&'static str>{let mut q=RadeonSubmissionQueue::new();let a=q.enqueue(64,1)?;let b=q.enqueue(128,2)?;q.mark_emitted(a)?;q.mark_emitted(b)?;if q.retire_head(1)?.map(|s|s.id)!=Some(a){return Err("C29 queue FIFO retire failed")}if q.retire_head(1)?.is_some(){return Err("C29 queue retired fence early")}if q.retire_head(2)?.map(|s|s.id)!=Some(b){return Err("C29 queue second retire failed")}let c=q.enqueue(256,3)?;q.mark_emitted(c)?;if q.cancel_all()!=1{return Err("C29 queue cancellation failed")}let (submitted,retired,cancelled,count)=q.counters();if count!=0||submitted!=3||retired!=2||cancelled!=1{return Err("C29 queue counters failed")}let fp=0xc029_5155_4555_0001u64^submitted^(retired<<16)^(cancelled<<32);let s=RadeonQueueState{ready:true,ordering_verified:true,cancellation_verified:true,submitted,retired,cancelled,fingerprint:fp};*STATE.lock()=s;Ok(s)}
pub fn state()->RadeonQueueState{*STATE.lock()}
