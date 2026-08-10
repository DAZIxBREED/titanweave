#![no_std]

use core::arch::asm;

pub use titanweave_forgeaudio_abi::*;

pub type Handle = u32;

pub const SYS_EXIT: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_CHANNEL_SEND: u64 = 2;
pub const SYS_CHANNEL_RECEIVE: u64 = 3;
pub const SYS_GETPID: u64 = 4;
pub const SYS_YIELD: u64 = 5;
pub const SYS_SYSTEM_QUERY: u64 = 6;
pub const SYS_AUDIO_ABI_QUERY: u64 = 44;
pub const SYS_AUDIO_ENUMERATE: u64 = 45;
pub const SYS_AUDIO_CONTROL: u64 = 46;

pub const RIGHT_READ: u32 = 1 << 0;
pub const RIGHT_WRITE: u32 = 1 << 1;
pub const INVALID_HANDLE: Handle = 0;

#[inline]
unsafe fn syscall0(number: u64) -> u64 {
    let result: u64;
    unsafe {
        asm!(
            "int 0x80",
            inlateout("rax") number => result,
            options(nostack),
        );
    }
    result
}

#[inline]
unsafe fn syscall3(number: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    let result: u64;
    unsafe {
        asm!(
            "int 0x80",
            inlateout("rax") number => result,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            options(nostack),
        );
    }
    result
}

#[inline]
unsafe fn syscall5(number: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64) -> u64 {
    let result: u64;
    unsafe {
        asm!(
            "int 0x80",
            inlateout("rax") number => result,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            in("r10") a4,
            in("r8") a5,
            options(nostack),
        );
    }
    result
}

pub fn getpid() -> u64 {
    unsafe { syscall0(SYS_GETPID) }
}

pub fn write(handle: Handle, bytes: &[u8]) -> i64 {
    unsafe { syscall3(SYS_WRITE, handle as u64, bytes.as_ptr() as u64, bytes.len() as u64) as i64 }
}

pub fn channel_send(
    handle: Handle,
    bytes: &[u8],
    transferred_handle: Handle,
    transferred_rights: u32,
) -> i64 {
    unsafe {
        syscall5(
            SYS_CHANNEL_SEND,
            handle as u64,
            bytes.as_ptr() as u64,
            bytes.len() as u64,
            transferred_handle as u64,
            transferred_rights as u64,
        ) as i64
    }
}

pub fn channel_receive(
    handle: Handle,
    bytes: &mut [u8],
    received_handle: Option<&mut Handle>,
) -> i64 {
    let out = received_handle
        .map(|handle| handle as *mut Handle as u64)
        .unwrap_or(0);
    unsafe {
        syscall5(
            SYS_CHANNEL_RECEIVE,
            handle as u64,
            bytes.as_mut_ptr() as u64,
            bytes.len() as u64,
            out,
            0,
        ) as i64
    }
}

pub fn yield_now() {
    let _ = unsafe { syscall0(SYS_YIELD) };
}

pub fn system_query(query: u64) -> i64 {
    unsafe { syscall3(SYS_SYSTEM_QUERY, query, 0, 0) as i64 }
}


pub fn audio_abi_query(info: &mut AudioAbiInfo) -> i64 {
    unsafe {
        syscall5(
            SYS_AUDIO_ABI_QUERY,
            info as *mut AudioAbiInfo as u64,
            core::mem::size_of::<AudioAbiInfo>() as u64,
            0,
            0,
            0,
        ) as i64
    }
}

pub fn audio_enumerate_device(index: u64, info: &mut AudioDeviceInfo) -> i64 {
    unsafe {
        syscall5(
            SYS_AUDIO_ENUMERATE,
            AudioObjectKind::Device as u64,
            0,
            index,
            info as *mut AudioDeviceInfo as u64,
            core::mem::size_of::<AudioDeviceInfo>() as u64,
        ) as i64
    }
}

pub fn audio_enumerate_endpoint(
    device_object_id: u64,
    index: u64,
    info: &mut AudioEndpointInfo,
) -> i64 {
    unsafe {
        syscall5(
            SYS_AUDIO_ENUMERATE,
            AudioObjectKind::Endpoint as u64,
            device_object_id,
            index,
            info as *mut AudioEndpointInfo as u64,
            core::mem::size_of::<AudioEndpointInfo>() as u64,
        ) as i64
    }
}

pub fn audio_control(
    request: &AudioControlRequest,
    response: &mut AudioControlResponse,
) -> i64 {
    unsafe {
        syscall5(
            SYS_AUDIO_CONTROL,
            request as *const AudioControlRequest as u64,
            core::mem::size_of::<AudioControlRequest>() as u64,
            response as *mut AudioControlResponse as u64,
            core::mem::size_of::<AudioControlResponse>() as u64,
            0,
        ) as i64
    }
}

pub const fn audio_pack_period_buffer(period_frames: u32, buffer_frames: u32) -> u64 {
    (period_frames as u64) | ((buffer_frames as u64) << 32)
}

pub fn exit(code: u64) -> ! {
    let _ = unsafe { syscall3(SYS_EXIT, code, 0, 0) };
    loop {
        core::hint::spin_loop();
    }
}
