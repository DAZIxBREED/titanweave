use crate::namespace::{self, NamespaceKind};
use crate::serial;
use crate::sync::SpinLock;
const MAX_SHARED_OBJECTS: usize = 32;
const SHARED_BYTES: usize = 4096;
#[derive(Clone, Copy)]
struct SharedObject { occupied: bool, object_id: u64, bytes: [u8; SHARED_BYTES], length: usize, readers: u32, writer: Option<u64> }
impl SharedObject { const EMPTY: Self = Self { occupied: false, object_id: 0, bytes: [0; SHARED_BYTES], length: 0, readers: 0, writer: None }; }
static OBJECTS: SpinLock<[SharedObject; MAX_SHARED_OBJECTS]> = SpinLock::new([SharedObject::EMPTY; MAX_SHARED_OBJECTS]);
const BOOT_STATUS_ID: u64 = 0x7000;

pub fn create(name: &[u8], object_id: u64) -> Result<(), &'static str> {
    let mut objects = OBJECTS.lock();
    if objects.iter().any(|object| object.occupied && object.object_id == object_id) { return Err("shared-memory object id already exists"); }
    let slot = objects.iter_mut().find(|object| !object.occupied).ok_or("shared-memory table is full")?;
    *slot = SharedObject { occupied: true, object_id, ..SharedObject::EMPTY };
    drop(objects);
    namespace::register(name, NamespaceKind::SharedMemory, object_id)
}
pub fn write(object_id: u64, owner: u64, input: &[u8]) -> Result<usize, &'static str> {
    if input.len() > SHARED_BYTES { return Err("shared-memory write exceeds object capacity"); }
    let mut objects = OBJECTS.lock();
    let object = objects.iter_mut().find(|object| object.occupied && object.object_id == object_id).ok_or("shared-memory object not found")?;
    if object.writer.is_some() && object.writer != Some(owner) { return Err("shared-memory object has another writer"); }
    object.writer = Some(owner); object.bytes[..input.len()].copy_from_slice(input); object.length = input.len();
    Ok(input.len())
}
pub fn read(object_id: u64, output: &mut [u8]) -> Result<usize, &'static str> {
    let mut objects = OBJECTS.lock();
    let object = objects.iter_mut().find(|object| object.occupied && object.object_id == object_id).ok_or("shared-memory object not found")?;
    object.readers = object.readers.saturating_add(1);
    let count = core::cmp::min(output.len(), object.length); output[..count].copy_from_slice(&object.bytes[..count]);
    object.readers -= 1; Ok(count)
}
pub fn initialize_core_objects() -> Result<(), &'static str> {
    create(b"\\Shared\\boot-status", BOOT_STATUS_ID)?;
    write(BOOT_STATUS_ID, 1, b"vfs=ready;services=starting")?;
    serial::println(format_args!("[SHM ] Shared-memory manager online"));
    Ok(())
}
pub fn status_bytes(output: &mut [u8]) -> usize { read(BOOT_STATUS_ID, output).unwrap_or(0) }
