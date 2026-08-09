//! First-fit I/O virtual-address allocator with splitting and coalescing.
pub const MAX_IOVA_EXTENTS:usize=256;
#[derive(Clone,Copy,Debug)]struct Extent{base:u64,pages:u64,free:bool}impl Extent{const EMPTY:Self=Self{base:0,pages:0,free:false};}
pub struct IovaAllocator{extents:[Extent;MAX_IOVA_EXTENTS],count:usize,page_size:u64}
impl IovaAllocator{
 pub const fn empty()->Self{Self{extents:[Extent::EMPTY;MAX_IOVA_EXTENTS],count:0,page_size:4096}}
 pub fn new(base:u64,pages:u64)->Result<Self,&'static str>{if pages==0||base&4095!=0{return Err("invalid IOVA aperture")}let mut s=Self::empty();s.extents[0]=Extent{base,pages,free:true};s.count=1;Ok(s)}
 pub fn allocate(&mut self,pages:u64,alignment_pages:u64)->Result<u64,&'static str>{if pages==0||!alignment_pages.is_power_of_two(){return Err("invalid IOVA request")}for i in 0..self.count{let e=self.extents[i];if !e.free{continue}let align=alignment_pages*self.page_size;let start=(e.base+align-1)&!(align-1);let skipped=(start-e.base)/self.page_size;if skipped+pages>e.pages{continue}let tail=e.pages-skipped-pages;if self.count+(skipped>0)as usize+(tail>0)as usize>MAX_IOVA_EXTENTS{return Err("IOVA extent table full")}let mut parts=[Extent::EMPTY;3];let mut n=0;if skipped>0{parts[n]=Extent{base:e.base,pages:skipped,free:true};n+=1}parts[n]=Extent{base:start,pages,free:false};n+=1;if tail>0{parts[n]=Extent{base:start+pages*self.page_size,pages:tail,free:true};n+=1}for j in (i+1..self.count).rev(){self.extents[j+n-1]=self.extents[j]}for j in 0..n{self.extents[i+j]=parts[j]}self.count=self.count+n-1;return Ok(start)}Err("IOVA aperture exhausted")}
 pub fn free(&mut self,base:u64,pages:u64)->Result<(),&'static str>{let i=self.extents[..self.count].iter().position(|e|!e.free&&e.base==base&&e.pages==pages).ok_or("IOVA allocation not found")?;self.extents[i].free=true;self.coalesce();Ok(())}
 fn coalesce(&mut self){let mut i=0;while i+1<self.count{let a=self.extents[i];let b=self.extents[i+1];if a.free&&b.free&&a.base+a.pages*self.page_size==b.base{self.extents[i].pages+=b.pages;for j in i+1..self.count-1{self.extents[j]=self.extents[j+1]}self.count-=1}else{i+=1}}}
}
