//! USB HID device state, report descriptors, keyboard and mouse event decoding.
use crate::usb_hid::{BootKeyboardState,KeyEvent};
pub const MAX_HID_REPORT:usize=64;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum HidProtocol{BootKeyboard,BootMouse,Report}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub struct MouseEvent{pub buttons:u8,pub dx:i8,pub dy:i8,pub wheel:i8}
pub struct HidDevice{pub address:u8,pub interface:u8,pub protocol:HidProtocol,pub report_bytes:u16,pub online:bool,keyboard:BootKeyboardState}
impl HidDevice{
 pub const fn new(address:u8,interface:u8,protocol:HidProtocol,report_bytes:u16)->Result<Self,&'static str>{if address==0||report_bytes as usize>MAX_HID_REPORT{return Err("invalid HID interface")}Ok(Self{address,interface,protocol,report_bytes,online:false,keyboard:BootKeyboardState::new()})}
 pub fn start(&mut self){self.online=true}
 pub fn stop(&mut self){self.online=false}
 pub fn decode_keyboard(&mut self,report:&[u8],emit:impl FnMut(KeyEvent))->Result<(),&'static str>{if !self.online||self.protocol!=HidProtocol::BootKeyboard{return Err("keyboard interface offline")}self.keyboard.decode(report,emit)}
 pub fn decode_mouse(&self,report:&[u8])->Result<MouseEvent,&'static str>{if !self.online||self.protocol!=HidProtocol::BootMouse{return Err("mouse interface offline")}if report.len()<3||report.len()>4{return Err("invalid boot mouse report")}Ok(MouseEvent{buttons:report[0],dx:report[1] as i8,dy:report[2] as i8,wheel:if report.len()==4{report[3] as i8}else{0}})}
}
