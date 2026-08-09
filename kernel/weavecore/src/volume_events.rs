#[derive(Clone,Copy)]
pub enum VolumeEventKind { Detected, Mounted, Unmounted, Changed, Locked, Unlocked, Dirty, RepairRequired, Removed, MountFailed, DriverMissing }
#[derive(Clone,Copy)] pub struct VolumeEvent { pub kind:VolumeEventKind,pub volume_id:[u8;16],pub generation:u64 }
pub struct EventRing { events:[Option<VolumeEvent>;32],head:usize,len:usize,generation:u64 }
impl EventRing { pub const fn new()->Self{Self{events:[None;32],head:0,len:0,generation:0}} pub fn push(&mut self,kind:VolumeEventKind,id:[u8;16]){self.generation=self.generation.wrapping_add(1);let at=(self.head+self.len)%self.events.len();self.events[at]=Some(VolumeEvent{kind,volume_id:id,generation:self.generation});if self.len<self.events.len(){self.len+=1}else{self.head=(self.head+1)%self.events.len()}} pub fn pop(&mut self)->Option<VolumeEvent>{if self.len==0{return None}let v=self.events[self.head].take();self.head=(self.head+1)%self.events.len();self.len-=1;v}}
