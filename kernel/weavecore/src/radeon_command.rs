//! K14.C31 typed command buffers and separate compute/graphics queues.
//!
//! The encoding is Titanweave's stable driver-internal command ABI used by the
//! QEMU reference backend and future hardware translators. It is deliberately
//! not presented as AMD PM4 until native GFX12 packet emission is qualified.

use core::ptr;
use crate::{memory::{FrameAllocator,FRAME_SIZE},radeon_memory::{self,RadeonMemoryObject}};
pub const RADEON_COMMAND_ABI_VERSION:u32=1;
pub const C31_COMMAND_BYTES:u64=16*1024;
pub const C31_QUEUE_DEPTH:usize=16;
pub const TW_CMD_MAGIC:u32=0x5457_434d;
#[repr(u16)]#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum CommandOpcode{BindPipeline=1,BindResource=2,Dispatch=3,DrawTriangle=4,Present=5,Fence=6}
#[repr(u8)]#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum QueueClass{Compute=1,Graphics=2}
#[derive(Clone,Copy,Debug)]pub struct DecodedCommand{pub opcode:CommandOpcode,pub a:u64,pub b:u64,pub c:u64,pub d:u64}
impl DecodedCommand{pub const EMPTY:Self=Self{opcode:CommandOpcode::Fence,a:0,b:0,c:0,d:0};}

pub struct CommandBuffer{owner:u64,object:RadeonMemoryObject,dwords:u32}
impl CommandBuffer{
 pub fn allocate(allocator:&mut FrameAllocator<'_>,owner:u64)->Result<Self,&'static str>{let o=radeon_memory::allocate_gtt(allocator,owner,C31_COMMAND_BYTES,FRAME_SIZE)?;radeon_memory::pin(owner,o.id,true)?;Ok(Self{owner,object:o,dwords:0})}
 fn push(&mut self,v:u32)->Result<(),&'static str>{if u64::from(self.dwords+1)*4>C31_COMMAND_BYTES{return Err("C31 command buffer overflow")};unsafe{ptr::write_volatile((self.object.kernel_virtual as *mut u32).add(self.dwords as usize),v)};self.dwords+=1;Ok(())}
 pub fn emit(&mut self,op:CommandOpcode,a:u64,b:u64,c:u64,d:u64)->Result<(),&'static str>{self.push(TW_CMD_MAGIC)?;self.push(op as u32)?;for v in [a,b,c,d]{self.push(v as u32)?;self.push((v>>32) as u32)?}Ok(())}
 pub fn object_id(&self)->u64{self.object.id}pub fn gpu_address(&self)->u64{self.object.gpu_virtual}pub fn kernel_address(&self)->u64{self.object.kernel_virtual}pub fn dwords(&self)->u32{self.dwords}
 pub fn decode(&self,index:u32)->Result<DecodedCommand,&'static str>{let base=index.checked_mul(10).ok_or("C31 command index overflow")?;if base+10>self.dwords{return Err("C31 command decode outside buffer")};let read=|i:u32|unsafe{ptr::read_volatile((self.object.kernel_virtual as *const u32).add((base+i) as usize))};if read(0)!=TW_CMD_MAGIC{return Err("C31 command magic invalid")};let op=match read(1){1=>CommandOpcode::BindPipeline,2=>CommandOpcode::BindResource,3=>CommandOpcode::Dispatch,4=>CommandOpcode::DrawTriangle,5=>CommandOpcode::Present,6=>CommandOpcode::Fence,_=>return Err("C31 command opcode invalid")};let pair=|i:u32|u64::from(read(i))|(u64::from(read(i+1))<<32);Ok(DecodedCommand{opcode:op,a:pair(2),b:pair(4),c:pair(6),d:pair(8)})}
 pub fn command_count(&self)->u32{self.dwords/10}
 pub fn release(self,allocator:&mut FrameAllocator<'_>)->Result<(),&'static str>{radeon_memory::pin(self.owner,self.object.id,false)?;radeon_memory::free(allocator,self.owner,self.object.id)}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]enum QueueStatus{Empty,Queued,Running,Retired,Cancelled}
#[derive(Clone,Copy,Debug)]struct QueueEntry{id:u64,buffer:u64,fence:u32,status:QueueStatus}
impl QueueEntry{const EMPTY:Self=Self{id:0,buffer:0,fence:0,status:QueueStatus::Empty};}
pub struct ExecutionQueue{class:QueueClass,entries:[QueueEntry;C31_QUEUE_DEPTH],head:usize,tail:usize,count:usize,next:u64,submitted:u64,retired:u64}
impl ExecutionQueue{
 pub const fn new(class:QueueClass)->Self{Self{class,entries:[QueueEntry::EMPTY;C31_QUEUE_DEPTH],head:0,tail:0,count:0,next:1,submitted:0,retired:0}}
 pub fn submit(&mut self,buffer:u64,fence:u32)->Result<u64,&'static str>{if buffer==0||fence==0||self.count==C31_QUEUE_DEPTH{return Err("C31 queue submission invalid/full")};let id=self.next;self.next=self.next.checked_add(1).ok_or("C31 queue id exhausted")?;self.entries[self.tail]=QueueEntry{id,buffer,fence,status:QueueStatus::Queued};self.tail=(self.tail+1)%C31_QUEUE_DEPTH;self.count+=1;self.submitted+=1;Ok(id)}
 pub fn start_head(&mut self)->Result<u64,&'static str>{if self.count==0{return Err("C31 queue empty")};let e=&mut self.entries[self.head];if e.status!=QueueStatus::Queued{return Err("C31 queue start transition invalid")};e.status=QueueStatus::Running;Ok(e.id)}
 pub fn retire_head(&mut self,fence:u32)->Result<u64,&'static str>{if self.count==0{return Err("C31 queue empty")};let e=self.entries[self.head];if e.status!=QueueStatus::Running||fence<e.fence{return Err("C31 queue retirement invalid")};let id=e.id;self.entries[self.head]=QueueEntry::EMPTY;self.head=(self.head+1)%C31_QUEUE_DEPTH;self.count-=1;self.retired+=1;Ok(id)}
 pub fn counters(&self)->(QueueClass,u64,u64,usize){(self.class,self.submitted,self.retired,self.count)}
}
pub fn queue_self_test()->Result<u64,&'static str>{let mut c=ExecutionQueue::new(QueueClass::Compute);let a=c.submit(11,1)?;if c.start_head()?!=a{return Err("C31 compute queue start failed")}if c.retire_head(1)?!=a{return Err("C31 compute queue retire failed")}let mut g=ExecutionQueue::new(QueueClass::Graphics);let b=g.submit(12,2)?;if g.start_head()?!=b||g.retire_head(2)?!=b{return Err("C31 graphics queue lifecycle failed")}let (_,cs,cr,cc)=c.counters();let(_,gs,gr,gc)=g.counters();if (cs,cr,cc,gs,gr,gc)!=(1,1,0,1,1,0){return Err("C31 queue counters failed")}Ok(0xc031_5155_4555_0001^cs^(cr<<8)^(gs<<16)^(gr<<24))}
