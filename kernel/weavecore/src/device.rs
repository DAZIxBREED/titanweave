//! K11 ForgeBus device model and registry.

pub const MAX_DEVICES: usize = 256;
pub const MAX_DEVICE_RESOURCES: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusType { Platform, Pci, Usb, Acpi, Virtual }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceState { Discovered, Authorized, Bound, Online, Suspended, Failed, Removed }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resource {
    None,
    IoPort { base: u16, length: u16 },
    Mmio { base: u64, length: u64, prefetchable: bool },
    Interrupt { vector: u8 },
    DmaMask { bits: u8 },
}

#[derive(Clone, Copy, Debug)]
pub struct Device {
    pub id: DeviceId,
    pub parent: Option<DeviceId>,
    pub bus: BusType,
    pub vendor_id: u16,
    pub product_id: u16,
    pub class_code: u8,
    pub subclass: u8,
    pub programming_interface: u8,
    pub revision: u8,
    pub location: u64,
    pub generation: u32,
    pub state: DeviceState,
    pub driver_id: Option<u64>,
    pub resources: [Resource; MAX_DEVICE_RESOURCES],
}
impl Device {
    pub const EMPTY: Self = Self { id: DeviceId(0), parent: None, bus: BusType::Virtual, vendor_id: 0, product_id: 0, class_code: 0, subclass: 0, programming_interface: 0, revision: 0, location: 0, generation: 0, state: DeviceState::Removed, driver_id: None, resources: [Resource::None; MAX_DEVICE_RESOURCES] };
}

pub struct DeviceRegistry { pub(crate) slots: [Device; MAX_DEVICES], next_id: u64, generation: u32 }
impl DeviceRegistry {
    pub const fn new() -> Self { Self { slots: [Device::EMPTY; MAX_DEVICES], next_id: 1, generation: 1 } }
    pub fn insert(&mut self, mut device: Device) -> Result<DeviceId, &'static str> {
        let slot = self.slots.iter_mut().find(|d| d.id.0 == 0 || d.state == DeviceState::Removed).ok_or("device registry full")?;
        let id = DeviceId(self.next_id); self.next_id = self.next_id.checked_add(1).ok_or("device id exhausted")?;
        device.id = id; device.generation = self.generation; device.state = DeviceState::Discovered; *slot = device; Ok(id)
    }
    pub fn get(&self, id: DeviceId) -> Option<&Device> { self.slots.iter().find(|d| d.id == id && d.state != DeviceState::Removed) }
    pub fn get_mut(&mut self, id: DeviceId) -> Option<&mut Device> { self.slots.iter_mut().find(|d| d.id == id && d.state != DeviceState::Removed) }
    pub fn remove(&mut self, id: DeviceId) -> Result<(), &'static str> { let d=self.get_mut(id).ok_or("device not found")?; d.state=DeviceState::Removed; d.driver_id=None; self.generation=self.generation.wrapping_add(1).max(1); Ok(()) }
    pub fn iter(&self) -> impl Iterator<Item=&Device> { self.slots.iter().filter(|d| d.id.0 != 0 && d.state != DeviceState::Removed) }
    pub fn count(&self) -> usize { self.iter().count() }
}
