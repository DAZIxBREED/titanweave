//! K14.C30 operational EDID parser and deterministic mode selection.
//!
//! The parser validates the base EDID header/checksum, decodes identity, and
//! extracts all non-zero base-block detailed timings into a bounded table.  It
//! is independent from the transport used to acquire the EDID (GOP fallback,
//! DDC, or DP AUX), so the same parser survives the later native transport path.

pub const EDID_BASE_BYTES: usize = 128;
pub const MAX_EDID_MODES: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdidMode {
    pub width: u32,
    pub height: u32,
    pub refresh_millihz: u32,
    pub pixel_clock_khz: u32,
    pub htotal: u32,
    pub vtotal: u32,
    pub preferred: bool,
}
impl EdidMode { pub const EMPTY: Self = Self { width:0,height:0,refresh_millihz:0,pixel_clock_khz:0,htotal:0,vtotal:0,preferred:false }; }

#[derive(Clone, Copy, Debug)]
pub struct EdidInfo {
    pub valid: bool,
    pub manufacturer: [u8;3],
    pub product_id: u16,
    pub serial: u32,
    pub week: u8,
    pub year: u16,
    pub extension_count: u8,
    pub mode_count: u8,
    pub modes: [EdidMode; MAX_EDID_MODES],
    pub fingerprint: u64,
}
impl EdidInfo { pub const EMPTY: Self = Self { valid:false,manufacturer:[0;3],product_id:0,serial:0,week:0,year:0,extension_count:0,mode_count:0,modes:[EdidMode::EMPTY;MAX_EDID_MODES],fingerprint:0 }; }

fn mix(mut h:u64,v:u64)->u64{h^=v;h=h.wrapping_mul(0x100000001b3);h}
fn vendor_char(v:u16,shift:u16)->u8 { let n=((v>>shift)&0x1f) as u8; if n==0 { b'?' } else { b'A'+n-1 } }

pub fn parse_base(block:&[u8;EDID_BASE_BYTES])->Result<EdidInfo,&'static str>{
    const HEADER:[u8;8]=[0x00,0xff,0xff,0xff,0xff,0xff,0xff,0x00];
    if block[..8]!=HEADER{return Err("EDID header invalid")}
    let mut sum=0u8;for b in block.iter(){sum=sum.wrapping_add(*b)}if sum!=0{return Err("EDID checksum invalid")}
    let raw_vendor=u16::from_be_bytes([block[8],block[9]]);
    let mut out=EdidInfo{valid:true,manufacturer:[vendor_char(raw_vendor,10),vendor_char(raw_vendor,5),vendor_char(raw_vendor,0)],
        product_id:u16::from_le_bytes([block[10],block[11]]),serial:u32::from_le_bytes([block[12],block[13],block[14],block[15]]),
        week:block[16],year:1990u16+u16::from(block[17]),extension_count:block[126],..EdidInfo::EMPTY};
    let mut count=0usize;
    for slot in 0..4usize {
        let off=54+slot*18; let d=&block[off..off+18];
        let pixel_10khz=u16::from_le_bytes([d[0],d[1]]) as u32;if pixel_10khz==0{continue}
        let hactive=u32::from(d[2]) | (u32::from(d[4]&0xf0)<<4);
        let hblank=u32::from(d[3]) | (u32::from(d[4]&0x0f)<<8);
        let vactive=u32::from(d[5]) | (u32::from(d[7]&0xf0)<<4);
        let vblank=u32::from(d[6]) | (u32::from(d[7]&0x0f)<<8);
        let htotal=hactive.checked_add(hblank).ok_or("EDID horizontal total overflow")?;
        let vtotal=vactive.checked_add(vblank).ok_or("EDID vertical total overflow")?;
        if hactive<320||vactive<200||htotal<=hactive||vtotal<=vactive{return Err("EDID detailed timing invalid")}
        let clock_khz=pixel_10khz*10;
        let denom=u64::from(htotal)*u64::from(vtotal);
        let refresh=((u64::from(clock_khz)*1_000_000)/denom) as u32;
        if refresh<10_000||refresh>1_000_000{return Err("EDID detailed timing refresh invalid")}
        if count<MAX_EDID_MODES { out.modes[count]=EdidMode{width:hactive,height:vactive,refresh_millihz:refresh,pixel_clock_khz:clock_khz,htotal,vtotal,preferred:slot==0};count+=1; }
    }
    if count==0{return Err("EDID has no usable detailed timing")}
    out.mode_count=count as u8;
    let mut fp=0xc030_4544_4944_0001u64;
    fp=mix(fp,u64::from(raw_vendor));fp=mix(fp,u64::from(out.product_id));fp=mix(fp,u64::from(out.serial));fp=mix(fp,u64::from(out.mode_count));
    for m in out.modes.iter().take(count){fp=mix(fp,(u64::from(m.width)<<32)|u64::from(m.height));fp=mix(fp,(u64::from(m.refresh_millihz)<<32)|u64::from(m.pixel_clock_khz));}
    out.fingerprint=fp;Ok(out)
}

pub fn choose_mode(info:&EdidInfo,target_width:u32,target_height:u32)->Option<EdidMode>{
    if !info.valid{return None}
    let count=usize::from(info.mode_count).min(MAX_EDID_MODES);
    for m in info.modes.iter().take(count){if m.width==target_width&&m.height==target_height{return Some(*m)}}
    for m in info.modes.iter().take(count){if m.preferred{return Some(*m)}}
    info.modes.iter().take(count).copied().next()
}

pub fn self_test()->Result<(EdidInfo,EdidMode),&'static str>{
    let mut b=[0u8;EDID_BASE_BYTES];b[..8].copy_from_slice(&[0x00,0xff,0xff,0xff,0xff,0xff,0xff,0x00]);
    // "TWN" packed as EDID 5-bit manufacturer characters.
    let vendor=((20u16)<<10)|((23u16)<<5)|14u16;let vb=vendor.to_be_bytes();b[8]=vb[0];b[9]=vb[1];b[10]=0x30;b[11]=0xc0;b[12..16].copy_from_slice(&0x3029_0001u32.to_le_bytes());b[16]=32;b[17]=36;
    // 2560x1440 ~60 Hz reduced-blanking detailed timing, 241.50 MHz.
    let d=&mut b[54..72];d[0..2].copy_from_slice(&24150u16.to_le_bytes());d[2]=0x00;d[3]=0xa0;d[4]=0xa0;d[5]=0xa0;d[6]=0x29;d[7]=0x50;
    // modest sync fields; parser does not require transport polarity for basic mode ownership.
    d[8]=48;d[9]=32;d[10]=3;d[11]=5;d[17]=0x1a;
    b[126]=0;let mut sum=0u8;for x in b[..127].iter(){sum=sum.wrapping_add(*x)}b[127]=0u8.wrapping_sub(sum);
    let info=parse_base(&b)?;let mode=choose_mode(&info,2560,1440).ok_or("EDID mode selection failed")?;
    if info.manufacturer!=*b"TWN"||mode.width!=2560||mode.height!=1440||!mode.preferred{return Err("EDID self-test decoded wrong mode")}
    Ok((info,mode))
}
