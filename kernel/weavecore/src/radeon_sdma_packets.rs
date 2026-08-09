//! Typed GFX12 / SDMA 7 packet encoding used by K14.C29.
//!
//! AMD's SDMA 7 implementation consumes the generated SDMA v6 packet format.
//! C29 exposes only typed COPY_LINEAR, FENCE and NOP encoders; no caller may
//! submit raw packet words through the userspace ABI.

pub const RADEON_SDMA_PACKET_ABI_VERSION:u32=1;
pub const SDMA_OP_NOP:u32=0;
pub const SDMA_OP_COPY:u32=1;
pub const SDMA_OP_FENCE:u32=5;
pub const SDMA_SUBOP_COPY_LINEAR:u32=0;
pub const SDMA_FENCE_MTYPE_UC:u32=3;
pub const COPY_LINEAR_DWORDS:usize=8;
pub const FENCE_DWORDS:usize=4;
pub const SDMA_COPY_MAX_BYTES:u32=0x0040_0000;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct CopyLinearPacket{pub words:[u32;COPY_LINEAR_DWORDS],pub source:u64,pub destination:u64,pub bytes:u32}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct FencePacket{pub words:[u32;FENCE_DWORDS],pub address:u64,pub sequence:u32}

pub const fn nop()->u32{SDMA_OP_NOP}

pub fn copy_linear(source:u64,destination:u64,bytes:u32)->Result<CopyLinearPacket,&'static str>{
 if bytes==0||bytes>SDMA_COPY_MAX_BYTES{return Err("SDMA copy byte count outside C29 bound")}
 if source.checked_add(bytes as u64).is_none()||destination.checked_add(bytes as u64).is_none(){return Err("SDMA copy address overflow")}
 // SDMA v7 buffer-copy encoding: OP=COPY, SUB_OP=LINEAR, CPV=1,
 // count is byte_count-1, endian/cache parameter word is zero, followed by
 // 64-bit source and destination and one zero CPV extension word.
 let header=SDMA_OP_COPY|(SDMA_SUBOP_COPY_LINEAR<<8)|(1u32<<19);
 Ok(CopyLinearPacket{words:[header,bytes-1,0,source as u32,(source>>32) as u32,destination as u32,(destination>>32) as u32,0],source,destination,bytes})
}

pub fn fence(address:u64,sequence:u32)->Result<FencePacket,&'static str>{
 if address==0||address&3!=0{return Err("SDMA fence address must be nonzero and dword aligned")}
 let header=SDMA_OP_FENCE|(SDMA_FENCE_MTYPE_UC<<16);
 Ok(FencePacket{words:[header,address as u32,(address>>32) as u32,sequence],address,sequence})
}

pub fn decode_copy(words:&[u32])->Result<(u64,u64,u32),&'static str>{
 if words.len()<COPY_LINEAR_DWORDS{return Err("short SDMA COPY_LINEAR packet")}
 if words[0]&0xff!=SDMA_OP_COPY||(words[0]>>8)&0xff!=SDMA_SUBOP_COPY_LINEAR{return Err("not a C29 SDMA COPY_LINEAR packet")}
 let bytes=(words[1]&0x3fff_ffff).checked_add(1).ok_or("SDMA copy count overflow")?;
 if bytes==0||bytes>SDMA_COPY_MAX_BYTES{return Err("decoded SDMA copy exceeds C29 bound")}
 let source=(words[3] as u64)|((words[4] as u64)<<32);let destination=(words[5] as u64)|((words[6] as u64)<<32);
 if words[2]!=0||words[7]!=0{return Err("C29 SDMA COPY_LINEAR parameter/extension word is not zero")}
 Ok((source,destination,bytes))
}

pub fn decode_fence(words:&[u32])->Result<(u64,u32),&'static str>{
 if words.len()<FENCE_DWORDS{return Err("short SDMA FENCE packet")}
 if words[0]&0xff!=SDMA_OP_FENCE||((words[0]>>16)&7)!=SDMA_FENCE_MTYPE_UC{return Err("not a C29 UC SDMA FENCE packet")}
 Ok(((words[1] as u64)|((words[2] as u64)<<32),words[3]))
}

pub fn self_test()->Result<u64,&'static str>{
 let c=copy_linear(0x1_0000_2000,0x1_0000_4000,4096)?;let (s,d,n)=decode_copy(&c.words)?;
 if s!=c.source||d!=c.destination||n!=4096||c.words[1]!=4095{return Err("SDMA COPY_LINEAR codec self-test failed")}
 let f=fence(0x1_0000_6000,0x29aa_55cc)?;let (a,q)=decode_fence(&f.words)?;
 if a!=f.address||q!=f.sequence{return Err("SDMA FENCE codec self-test failed")}
 Ok((c.words[0] as u64)^((f.words[0] as u64)<<32)^s^d^a^u64::from(q))
}
