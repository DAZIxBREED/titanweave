use crate::block::MemoryBlockDevice;
use crate::{gpt, ntfs, serial, volume};

pub const MAX_MOUNTS:usize=32;
#[derive(Clone,Copy)]
pub struct MountedVolume { pub id:[u8;16], pub first_lba:u64, pub sectors:u64, pub filesystem:volume::FilesystemKind, pub policy:volume::MountPolicy, pub trust:volume::TrustClass, pub role:volume::VolumeRole, pub alias:u8, pub active:bool }
impl MountedVolume { pub const EMPTY:Self=Self{id:[0;16],first_lba:0,sectors:0,filesystem:volume::FilesystemKind::Unknown,policy:volume::MountPolicy::Quarantine,trust:volume::TrustClass::Quarantined,role:volume::VolumeRole::Unknown,alias:0,active:false}; }
#[derive(Clone,Copy)]
pub struct AutoMountReport { pub discovered:usize,pub mounted:usize,pub read_only:usize,pub hidden:usize,pub quarantined:usize,pub registry:[MountedVolume;MAX_MOUNTS] }

pub fn discover(device:MemoryBlockDevice)->Result<AutoMountReport,&'static str>{
 let mut out=AutoMountReport{discovered:0,mounted:0,read_only:0,hidden:0,quarantined:0,registry:[MountedVolume::EMPTY;MAX_MOUNTS]};
 match gpt::inspect(device) {
  Ok(table)=>{
   for index in 0..table.discovered_count {
    let part=table.partitions[index]; let role=role_from_gpt(part.type_guid,&part);
    let view=device.slice(part.first_lba,part.sector_count())?;
    let mut probe=volume::probe_with_role(view,role,false)?;
    if probe.filesystem==volume::FilesystemKind::Ntfs {
      let state=ntfs::inspect(view)?.safety;
      probe.policy=match state {
       ntfs::NtfsSafetyState::CleanReadOnly
       |ntfs::NtfsSafetyState::DirtyReadOnly
       |ntfs::NtfsSafetyState::HibernatedReadOnly=>volume::MountPolicy::ReadOnly,
       ntfs::NtfsSafetyState::NotNtfs
       |ntfs::NtfsSafetyState::Invalid=>volume::MountPolicy::Quarantine,
      };
    }
    register(&mut out,MountedVolume{id:part.unique_guid,first_lba:part.first_lba,sectors:part.sector_count(),filesystem:probe.filesystem,policy:probe.policy,trust:probe.trust,role:probe.role,alias:alias_for(index,probe.role),active:probe.policy!=volume::MountPolicy::Quarantine})?;
   }
  }
  Err(_)=>{ let probe=volume::probe_with_role(device,volume::VolumeRole::Recovery,false)?; register(&mut out,MountedVolume{id:[0;16],first_lba:0,sectors:device.sector_count(),filesystem:probe.filesystem,policy:probe.policy,trust:probe.trust,role:probe.role,alias:b'R',active:true})?; }
 }
 for n in 0..out.discovered { let v=out.registry[n]; serial::println(format_args!("[MNT ] alias={} role={} fs={} policy={} trust={} lba={} sectors={}",if v.alias==0{'-'}else{v.alias as char},v.role.name(),v.filesystem.name(),v.policy.name(),v.trust.name(),v.first_lba,v.sectors)); }
 Ok(out)
}
fn register(out:&mut AutoMountReport,v:MountedVolume)->Result<(),&'static str>{if out.discovered==MAX_MOUNTS{return Err("K9 mount registry is full");} out.registry[out.discovered]=v;out.discovered+=1;match v.policy{volume::MountPolicy::ReadWrite=>out.mounted+=1,volume::MountPolicy::ReadOnly=>{out.mounted+=1;out.read_only+=1},volume::MountPolicy::Hidden=>out.hidden+=1,volume::MountPolicy::Quarantine=>out.quarantined+=1,volume::MountPolicy::Locked=>{}}Ok(())}
fn alias_for(index:usize,role:volume::VolumeRole)->u8{match role{volume::VolumeRole::System=>b'S',volume::VolumeRole::Recovery=>b'R',volume::VolumeRole::Efi=>0,_=>b'D'.saturating_add(index as u8)}}
fn role_from_gpt(type_guid:[u8;16],part:&gpt::GptPartition)->volume::VolumeRole{
 const EFI:[u8;16]=[0x28,0x73,0x2a,0xc1,0x1f,0xf8,0xd2,0x11,0xba,0x4b,0x00,0xa0,0xc9,0x3e,0xc9,0x3b];
 if type_guid==EFI{return volume::VolumeRole::Efi}
 let mut name=[0u8;36];let len=part.ascii_name(&mut name);let n=&name[..len];
 if contains_ci(n,b"RECOVERY"){volume::VolumeRole::Recovery}else if contains_ci(n,b"SYSTEM"){volume::VolumeRole::System}else{volume::VolumeRole::Data}
}
fn contains_ci(h:&[u8],n:&[u8])->bool{if n.is_empty()||n.len()>h.len(){return false} for s in 0..=h.len()-n.len(){let mut ok=true;for i in 0..n.len(){if h[s+i].to_ascii_uppercase()!=n[i].to_ascii_uppercase(){ok=false;break}}if ok{return true}}false}
