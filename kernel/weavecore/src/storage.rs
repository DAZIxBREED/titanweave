use crate::block::MemoryBlockDevice;
use crate::{automount, serial};
use titanweave_boot_protocol::{boot_module_kind,BootInfo};
#[derive(Clone,Copy)] pub struct StorageReport{pub sectors:u64,pub discovered:usize,pub mounted:usize,pub read_only:usize,pub hidden:usize,pub quarantined:usize}
pub fn initialize(boot_info:&BootInfo)->Result<StorageReport,&'static str>{
 let module=boot_info.modules.iter().find(|m|m.kind==boot_module_kind::BOOT_VOLUME).ok_or("BootInfo contains no K9 bootstrap volume")?;
 let byte_len=usize::try_from(module.byte_size).map_err(|_|"boot volume size overflow")?;let device=MemoryBlockDevice::new(module.physical_address,byte_len)?;
 let mounts=automount::discover(device)?;
 serial::println(format_args!("[AUTO] compatible-volume pass: discovered={} mounted={} ro={} hidden={} quarantined={}",mounts.discovered,mounts.mounted,mounts.read_only,mounts.hidden,mounts.quarantined));
 Ok(StorageReport{sectors:device.sector_count(),discovered:mounts.discovered,mounted:mounts.mounted,read_only:mounts.read_only,hidden:mounts.hidden,quarantined:mounts.quarantined})
}
