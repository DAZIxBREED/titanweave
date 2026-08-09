pub type Handle = u32;
pub const CONSOLE_HANDLE: Handle = 1;
pub const SERVICE_CHANNEL_HANDLE: Handle = 2;
pub const DISPLAY_PRESENT_HANDLE: Handle = 3;
pub const GRAPHICS_PRESENT_OBJECT_ID: u64 = 0x7001;
pub const RIGHT_READ: u32 = 1 << 0;
pub const RIGHT_WRITE: u32 = 1 << 1;
pub const RIGHT_TRANSFER: u32 = 1 << 2;
pub const RIGHT_DUPLICATE: u32 = 1 << 3;
pub const RIGHT_CLOSE: u32 = 1 << 4;
pub const MAX_HANDLES: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleObject {
    None,
    Console,
    ChannelEndpoint { channel: u8, side: u8 },
    File { object_id: u64 },
    Process { object_id: u64 },
    SharedMemory { object_id: u64 },
    Device { object_id: u64 },
}
#[derive(Clone, Copy, Debug)]
pub struct HandleDescriptor { pub object: HandleObject, pub rights: u32 }
#[derive(Clone, Copy)]
struct HandleEntry { object: HandleObject, rights: u32 }
impl HandleEntry { const EMPTY: Self = Self { object: HandleObject::None, rights: 0 }; }

pub struct HandleTable { entries: [HandleEntry; MAX_HANDLES], used: usize }
impl HandleTable {
    pub const fn new() -> Self { Self { entries: [HandleEntry::EMPTY; MAX_HANDLES], used: 0 } }
    pub fn install(&mut self, handle: Handle, object: HandleObject, rights: u32) -> Result<(), &'static str> {
        let index = usize::try_from(handle).map_err(|_| "handle index overflow")?;
        if index == 0 || index >= MAX_HANDLES || object == HandleObject::None || rights == 0 { return Err("invalid handle installation"); }
        if self.entries[index].object != HandleObject::None { return Err("handle slot already occupied"); }
        self.entries[index] = HandleEntry { object, rights };
        self.used += 1;
        Ok(())
    }
    pub fn allocate(&mut self, object: HandleObject, rights: u32) -> Result<Handle, &'static str> {
        if object == HandleObject::None || rights == 0 { return Err("invalid handle allocation"); }
        for index in 1..MAX_HANDLES {
            if self.entries[index].object == HandleObject::None {
                self.entries[index] = HandleEntry { object, rights };
                self.used += 1;
                return u32::try_from(index).map_err(|_| "handle index overflow");
            }
        }
        Err("process handle table is full")
    }
    pub fn close(&mut self, handle: Handle) -> Result<HandleDescriptor, &'static str> {
        let index = usize::try_from(handle).map_err(|_| "handle index overflow")?;
        if index == 0 || index >= MAX_HANDLES { return Err("invalid process handle"); }
        let entry = self.entries[index];
        if entry.object == HandleObject::None { return Err("unbound process handle"); }
        self.entries[index] = HandleEntry::EMPTY;
        self.used -= 1;
        Ok(HandleDescriptor { object: entry.object, rights: entry.rights })
    }
    pub fn lookup(&self, handle: Handle, required_rights: u32) -> Result<HandleObject, &'static str> {
        Ok(self.describe(handle, required_rights)?.object)
    }
    pub fn describe(&self, handle: Handle, required_rights: u32) -> Result<HandleDescriptor, &'static str> {
        let index = usize::try_from(handle).map_err(|_| "handle index overflow")?;
        let Some(entry) = self.entries.get(index).copied() else { return Err("invalid process handle"); };
        if entry.object == HandleObject::None { return Err("unbound process handle"); }
        if entry.rights & required_rights != required_rights { return Err("handle does not grant required rights"); }
        Ok(HandleDescriptor { object: entry.object, rights: entry.rights })
    }
    pub fn transferable(&self, handle: Handle, requested_rights: u32) -> Result<HandleDescriptor, &'static str> {
        let descriptor = self.describe(handle, RIGHT_TRANSFER)?;
        if requested_rights == 0 || requested_rights & !descriptor.rights != 0 { return Err("transferred rights exceed source handle rights"); }
        Ok(HandleDescriptor { object: descriptor.object, rights: requested_rights })
    }
    #[must_use] pub const fn used(&self) -> usize { self.used }
}
impl HandleTable {
    pub fn close_all(&mut self, mut release: impl FnMut(HandleDescriptor)) -> usize {
        let mut closed=0;
        for index in 1..MAX_HANDLES {
            let entry=self.entries[index];
            if entry.object!=HandleObject::None {
                self.entries[index]=HandleEntry::EMPTY;
                release(HandleDescriptor{object:entry.object,rights:entry.rights});
                closed+=1;
            }
        }
        self.used=0;
        closed
    }
}
