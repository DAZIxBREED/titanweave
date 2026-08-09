//! Segment-aware PCI identities used by IOMMU, MSI and hot-plug backends.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct PciAddress{pub segment:u16,pub bus:u8,pub device:u8,pub function:u8}
impl PciAddress{
 pub const fn new(segment:u16,bus:u8,device:u8,function:u8)->Result<Self,&'static str>{if device>=32||function>=8{return Err("invalid PCI address")}Ok(Self{segment,bus,device,function})}
 pub const fn requester_id(self)->RequesterId{RequesterId(((self.bus as u16)<<8)|((self.device as u16)<<3)|(self.function as u16))}
 pub const fn packed(self)->u64{((self.segment as u64)<<32)|((self.bus as u64)<<16)|((self.device as u64)<<8)|self.function as u64}
 pub const fn from_packed(v:u64)->Self{Self{segment:(v>>32)as u16,bus:(v>>16)as u8,device:(v>>8)as u8,function:v as u8}}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub struct RequesterId(pub u16);
