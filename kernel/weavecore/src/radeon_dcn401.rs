//! K14.C30 source-reviewed DCN 4.01 resource model.
//!
//! Linux/AMD DCN401 exposes four timing generators, OPPs, video planes, audio
//! engines, stream encoders, HPO DP stream/link encoders, PLLs, DDC engines,
//! MPC 3D LUTs and DSC engines, plus one display-writeback block and 16 VMIDs.
//! C30 uses this as a bounded capability description.  It does not invent MMIO
//! offsets or claim native DCN programming before those registers/transports
//! are individually reviewed and their prerequisites are live.

pub const DCN401_TIMING_GENERATORS:u8=4;
pub const DCN401_OPPS:u8=4;
pub const DCN401_VIDEO_PLANES:u8=4;
pub const DCN401_AUDIO:u8=4;
pub const DCN401_STREAM_ENCODERS:u8=4;
pub const DCN401_HPO_DP_STREAM_ENCODERS:u8=4;
pub const DCN401_HPO_DP_LINK_ENCODERS:u8=4;
pub const DCN401_PLLS:u8=4;
pub const DCN401_DWB:u8=1;
pub const DCN401_DDC:u8=4;
pub const DCN401_VMIDS:u8=16;
pub const DCN401_MPC_3DLUT:u8=4;
pub const DCN401_DSC:u8=4;
pub const DCN401_I2C_KHZ:u32=95;

#[derive(Clone,Copy,Debug)]
pub struct Dcn401Caps{pub timing_generators:u8,pub planes:u8,pub stream_encoders:u8,pub ddc:u8,pub dsc:u8,pub audio:u8,pub source_reviewed:bool,pub fingerprint:u64}
fn mix(mut h:u64,v:u64)->u64{h^=v;h=h.wrapping_mul(0x100000001b3);h}
pub fn capabilities()->Result<Dcn401Caps,&'static str>{
 if DCN401_TIMING_GENERATORS!=4||DCN401_OPPS!=4||DCN401_VIDEO_PLANES!=4||DCN401_AUDIO!=4||DCN401_STREAM_ENCODERS!=4||DCN401_HPO_DP_STREAM_ENCODERS!=4||DCN401_HPO_DP_LINK_ENCODERS!=4||DCN401_PLLS!=4||DCN401_DWB!=1||DCN401_DDC!=4||DCN401_VMIDS!=16||DCN401_MPC_3DLUT!=4||DCN401_DSC!=4||DCN401_I2C_KHZ!=95{return Err("DCN401 source-reviewed resource constants changed")}
 let mut fp=0xc030_4443_4e34_0101u64;for v in [4u64,4,4,4,4,4,4,4,1,4,16,4,4,95]{fp=mix(fp,v)}
 Ok(Dcn401Caps{timing_generators:4,planes:4,stream_encoders:4,ddc:4,dsc:4,audio:4,source_reviewed:true,fingerprint:fp})
}
