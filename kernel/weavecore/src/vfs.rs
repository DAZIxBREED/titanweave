use crate::block::MemoryBlockDevice;
use crate::fat32::{DirectoryEntry, Fat32Volume};
use crate::serial;
use crate::sync::SpinLock;
use titanweave_boot_protocol::{boot_module_kind, BootInfo};

const FILE_SCRATCH_BYTES: usize = 128 * 1024;

struct MountedVolume {
    volume: Fat32Volume,
}

static VOLUME: SpinLock<Option<MountedVolume>> = SpinLock::new(None);
static FILE_SCRATCH: SpinLock<[u8; FILE_SCRATCH_BYTES]> =
    SpinLock::new([0; FILE_SCRATCH_BYTES]);

pub fn mount_boot_volume(boot_info: &BootInfo) -> Result<(), &'static str> {
    let module = boot_info
        .modules
        .iter()
        .find(|module| module.kind == boot_module_kind::BOOT_VOLUME)
        .ok_or("BootInfo contains no bootstrap volume")?;
    let byte_len = usize::try_from(module.byte_size).map_err(|_| "boot volume size overflow")?;
    let device = MemoryBlockDevice::new(module.physical_address, byte_len)?;
    let volume = Fat32Volume::mount(device)?;
    *VOLUME.lock() = Some(MountedVolume { volume });
    serial::println(format_args!(
        "[VFS ] FAT32 bootstrap volume mounted as C: clusters={} bytes",
        volume.bytes_per_cluster()
    ));
    Ok(())
}

pub fn with_file<R>(
    path: &[u8],
    operation: impl FnOnce(&[u8]) -> Result<R, &'static str>,
) -> Result<R, &'static str> {
    let volume = {
        let mounted = VOLUME.lock();
        mounted.as_ref().ok_or("VFS boot volume is not mounted")?.volume
    };
    let mut scratch = FILE_SCRATCH.lock();
    let length = volume.read_file(path, &mut scratch[..])?;
    operation(&scratch[..length])
}

pub fn file_exists(path: &[u8]) -> bool {
    let mounted = VOLUME.lock();
    let Some(mounted) = mounted.as_ref() else { return false };
    mounted.volume.lookup(path).is_ok()
}

pub fn log_directory(path: &[u8]) -> Result<usize, &'static str> {
    let volume = {
        let mounted = VOLUME.lock();
        mounted.as_ref().ok_or("VFS boot volume is not mounted")?.volume
    };
    let mut count = 0usize;
    volume.visit_directory(path, |entry| {
        count += 1;
        log_entry(entry);
    })?;
    Ok(count)
}

fn log_entry(entry: DirectoryEntry) {
    let mut display = [b' '; 12];
    let mut length = 0usize;
    for &byte in &entry.short_name[..8] {
        if byte == b' ' { break; }
        display[length] = byte;
        length += 1;
    }
    if entry.short_name[8] != b' ' {
        display[length] = b'.';
        length += 1;
        for &byte in &entry.short_name[8..] {
            if byte == b' ' { break; }
            display[length] = byte;
            length += 1;
        }
    }
    serial::println(format_args!(
        "[DIR ] {} {} bytes cluster={}",
        core::str::from_utf8(&display[..length]).unwrap_or("?"),
        entry.byte_size,
        entry.first_cluster
    ));
}
