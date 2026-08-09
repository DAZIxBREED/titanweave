use crate::{arch::x86_64::port::{inl,outl},device::Resource};
const CONFIG_ADDRESS:u16=0x0cf8;const CONFIG_DATA:u16=0x0cfc;
#[derive(Clone,Copy,Debug)]pub struct PciFunction{pub bus:u8,pub device:u8,pub function:u8,pub vendor_id:u16,pub device_id:u16,pub class_code:u8,pub subclass:u8,pub programming_interface:u8,pub revision:u8,pub header_type:u8}
pub fn enumerate(mut visit:impl FnMut(PciFunction)){for bus in 0u16..=255{for device in 0u8..32{let identity=read_u32(bus as u8,device,0,0);if identity as u16==0xffff{continue}let header=read_u32(bus as u8,device,0,0x0c);let functions=if header&(1<<23)!=0{8}else{1};for function in 0u8..functions{let identity=read_u32(bus as u8,device,function,0);if identity as u16==0xffff{continue}let class=read_u32(bus as u8,device,function,0x08);visit(PciFunction{bus:bus as u8,device,function,vendor_id:identity as u16,device_id:(identity>>16)as u16,class_code:(class>>24)as u8,subclass:(class>>16)as u8,programming_interface:(class>>8)as u8,revision:class as u8,header_type:(read_u32(bus as u8,device,function,0x0c)>>16)as u8})}}}}
pub fn find_first(mut predicate:impl FnMut(PciFunction)->bool)->Option<PciFunction>{let mut found=None;enumerate(|f|{if found.is_none()&&predicate(f){found=Some(f)}});found}
pub fn read_u32(bus:u8,device:u8,function:u8,offset:u8)->u32{let address=0x8000_0000u32|((bus as u32)<<16)|((device as u32)<<11)|((function as u32)<<8)|((offset as u32)&0xfc);unsafe{outl(CONFIG_ADDRESS,address);inl(CONFIG_DATA)}}
pub fn write_u32(bus:u8,device:u8,function:u8,offset:u8,value:u32){let address=0x8000_0000u32|((bus as u32)<<16)|((device as u32)<<11)|((function as u32)<<8)|((offset as u32)&0xfc);unsafe{outl(CONFIG_ADDRESS,address);outl(CONFIG_DATA,value)}}
pub fn enable_memory_and_bus_master(function:PciFunction){let value=read_u32(function.bus,function.device,function.function,0x04);write_u32(function.bus,function.device,function.function,0x04,value|(1<<1)|(1<<2));}
pub fn enable_io_and_bus_master(function:PciFunction){let value=read_u32(function.bus,function.device,function.function,0x04);write_u32(function.bus,function.device,function.function,0x04,value|(1<<0)|(1<<2));}
pub fn read_resources(f:PciFunction,out:&mut[Resource;8]){let mut slot=0usize;let bar_count=if f.header_type&0x7f==0{6}else{2};let mut index=0;while index<bar_count&&slot<out.len(){let off=0x10+(index*4)as u8;let raw=read_u32(f.bus,f.device,f.function,off);if raw==0||raw==0xffff_ffff{index+=1;continue}if raw&1==1{out[slot]=Resource::IoPort{base:(raw&0xfffc)as u16,length:0};slot+=1;index+=1;continue}let kind=(raw>>1)&3;let prefetchable=raw&(1<<3)!=0;let mut base=(raw&0xffff_fff0)as u64;if kind==2&&index+1<bar_count{base|=(read_u32(f.bus,f.device,f.function,off+4)as u64)<<32;index+=1}out[slot]=Resource::Mmio{base,length:0,prefetchable};slot+=1;index+=1;}let irq=(read_u32(f.bus,f.device,f.function,0x3c)&0xff)as u8;if irq!=0&&irq!=0xff&&slot<out.len(){out[slot]=Resource::Interrupt{vector:irq};}}

/// Read a byte from conventional PCI configuration space.
pub fn read_u8(bus:u8,device:u8,function:u8,offset:u8)->u8{
 let value=read_u32(bus,device,function,offset&0xfc);
 ((value>>(((offset&3)as u32)*8))&0xff)as u8
}
/// Read a little-endian 16-bit value from conventional PCI configuration space.
pub fn read_u16(bus:u8,device:u8,function:u8,offset:u8)->u16{
 let lo=read_u8(bus,device,function,offset)as u16;
 let hi=read_u8(bus,device,function,offset.wrapping_add(1))as u16;
 lo|(hi<<8)
}
/// Write a little-endian 16-bit value to conventional PCI configuration space.
/// This uses a 16-bit CONFIG_DATA access so the adjacent PCI Status word is
/// not rewritten (important because some Status bits are write-one-to-clear).
pub fn write_u16(bus:u8,device:u8,function:u8,offset:u8,value:u16){
 use crate::arch::x86_64::port::outw;
 let aligned=offset&0xfc;
 let lane=(offset&2)as u16;
 let address=0x8000_0000u32|((bus as u32)<<16)|((device as u32)<<11)|((function as u32)<<8)|((aligned as u32)&0xfc);
 unsafe{outl(CONFIG_ADDRESS,address);outw(CONFIG_DATA+lane,value)}
}
/// Read the base address of a memory BAR. I/O BARs return `None`.
pub fn memory_bar_base(function:PciFunction,index:u8)->Option<u64>{
 if index>=6{return None}
 let offset=0x10u8.checked_add(index.checked_mul(4)?)?;
 let raw=read_u32(function.bus,function.device,function.function,offset);
 if raw==0||raw==0xffff_ffff||raw&1!=0{return None}
 let kind=(raw>>1)&3;
 let mut base=(raw&0xffff_fff0)as u64;
 if kind==2{
  if index>=5{return None}
  base|=(read_u32(function.bus,function.device,function.function,offset+4)as u64)<<32;
 }
 Some(base)
}
/// Enable memory decoding without yet authorizing DMA bus mastering.
pub fn enable_memory_decode(function:PciFunction){
 let value=read_u32(function.bus,function.device,function.function,0x04);
 write_u32(function.bus,function.device,function.function,0x04,value|(1<<1));
}
/// Enable PCI bus mastering after the driver has established bounded DMA ownership.
pub fn enable_bus_master(function:PciFunction){
 let value=read_u32(function.bus,function.device,function.function,0x04);
 write_u32(function.bus,function.device,function.function,0x04,value|(1<<2));
}
/// Disable PCI bus mastering before DMA mappings are revoked or a device is reset.
pub fn disable_bus_master(function:PciFunction){
 let value=read_u32(function.bus,function.device,function.function,0x04);
 write_u32(function.bus,function.device,function.function,0x04,value&!(1<<2));
}
