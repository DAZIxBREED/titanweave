//! K11 IOMMU policy boundary. Hardware backends attach to this contract.
use crate::device::DeviceId;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum IommuMode{Unavailable,Passthrough,Translated,Strict}
pub trait IommuBackend{fn mode(&self)->IommuMode;fn attach(&mut self,device:DeviceId,domain:u16)->Result<(),&'static str>;fn detach(&mut self,device:DeviceId)->Result<(),&'static str>;fn invalidate(&mut self,domain:u16)->Result<(),&'static str>;}
pub struct IommuPolicy{pub require_translation_for_external:bool,pub allow_passthrough_for_platform:bool}impl IommuPolicy{pub const fn secure_default()->Self{Self{require_translation_for_external:true,allow_passthrough_for_platform:true}}pub fn authorize(&self,mode:IommuMode,external:bool)->Result<(),&'static str>{if external&&self.require_translation_for_external&&!matches!(mode,IommuMode::Translated|IommuMode::Strict){return Err("external DMA device requires translated IOMMU domain")}Ok(())}}
