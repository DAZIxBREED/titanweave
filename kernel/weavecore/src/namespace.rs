use crate::serial;
use crate::sync::SpinLock;
const MAX_ENTRIES: usize = 128;
const NAME_BYTES: usize = 96;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamespaceKind { Service, Device, SharedMemory, Channel, Volume, FileSystem }
#[derive(Clone, Copy)]
struct Entry { occupied: bool, name: [u8; NAME_BYTES], length: usize, kind: NamespaceKind, object_id: u64 }
impl Entry { const EMPTY: Self = Self { occupied: false, name: [0; NAME_BYTES], length: 0, kind: NamespaceKind::Service, object_id: 0 }; }
static ENTRIES: SpinLock<[Entry; MAX_ENTRIES]> = SpinLock::new([Entry::EMPTY; MAX_ENTRIES]);

pub fn register(name: &[u8], kind: NamespaceKind, object_id: u64) -> Result<(), &'static str> {
    validate_name(name)?;
    if object_id == 0 { return Err("namespace object id is invalid"); }
    let mut entries = ENTRIES.lock();
    if entries.iter().any(|entry| entry.occupied && &entry.name[..entry.length] == name) { return Err("namespace name is already registered"); }
    let slot = entries.iter_mut().find(|entry| !entry.occupied).ok_or("namespace is full")?;
    slot.occupied = true; slot.name[..name.len()].copy_from_slice(name); slot.length = name.len(); slot.kind = kind; slot.object_id = object_id;
    Ok(())
}
pub fn unregister(name: &[u8], expected_object_id: u64) -> Result<(), &'static str> {
    validate_name(name)?;
    let mut entries = ENTRIES.lock();
    let slot = entries.iter_mut().find(|entry| entry.occupied && &entry.name[..entry.length] == name).ok_or("namespace name is not registered")?;
    if slot.object_id != expected_object_id { return Err("namespace object identity mismatch"); }
    *slot = Entry::EMPTY;
    Ok(())
}
pub fn lookup(name: &[u8], kind: NamespaceKind) -> Option<u64> {
    ENTRIES.lock().iter().find(|entry| entry.occupied && entry.kind == kind && &entry.name[..entry.length] == name).map(|entry| entry.object_id)
}
pub fn initialize_core_namespace() -> Result<(), &'static str> {
    register(b"\\Devices\\Block0", NamespaceKind::Device, 0x6000)?;
    register(b"\\Services\\init", NamespaceKind::Service, 0x6001)?;
    register(b"\\Services\\logging", NamespaceKind::Service, 0x6002)?;
    register(b"\\Services\\console", NamespaceKind::Service, 0x6003)?;
    register(b"\\Services\\archive", NamespaceKind::Service, 0x6004)?;
    register(b"\\Services\\shell", NamespaceKind::Service, 0x6005)?;
    register(b"\\Services\\display", NamespaceKind::Service, 0x6006)?;
    register(b"\\Channels\\system-log", NamespaceKind::Channel, 0x6010)?;
    serial::println(format_args!("[NAME] Object namespace initialized"));
    Ok(())
}
pub fn log_services() {
    let entries = ENTRIES.lock();
    for entry in entries.iter().filter(|entry| entry.occupied && entry.kind == NamespaceKind::Service) {
        serial::println(format_args!("[SVC ] {} object={:#x}", core::str::from_utf8(&entry.name[..entry.length]).unwrap_or("service"), entry.object_id));
    }
}
fn validate_name(name: &[u8]) -> Result<(), &'static str> {
    if name.len() < 2 || name.len() > NAME_BYTES || name[0] != b'\\' || name.iter().any(|byte| *byte == 0) { return Err("invalid namespace path"); }
    Ok(())
}
