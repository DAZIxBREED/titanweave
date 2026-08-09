use crate::abi::*;
use crate::handles::{Handle, HandleObject, RIGHT_READ, RIGHT_WRITE};
use crate::ipc::MAX_MESSAGE_BYTES;
use crate::user::UserTrapFrame;
use crate::{display, gpu_runtime, native_gpu, native_gpu_binding, native_gpu_c2, native_gpu_c3, native_gpu_c4, native_gpu_c5, native_gpu_c6, native_gpu_c7, native_gpu_c8, native_gpu_c9, native_gpu_c10, native_gpu_c11, native_gpu_c12, native_gpu_c13, native_gpu_c14, native_gpu_c15, native_gpu_c16, native_gpu_c17, native_gpu_c18, native_gpu_c19, native_gpu_c20, native_gpu_c21, native_gpu_c22, native_gpu_c23, native_gpu_c24, native_gpu_c25, native_gpu_c26, native_gpu_c27, native_gpu_c28, native_gpu_c29, native_gpu_c30, native_gpu_c31, namespace, percpu, process, serial, shared_memory, vfs};

#[unsafe(no_mangle)]
pub extern "C" fn weave_syscall_dispatch(frame: *mut UserTrapFrame) -> *mut UserTrapFrame {
    let (number, a1, a2, a3, a4, a5, vector, cs) = unsafe {
        let frame_ref = &*frame;
        (
            frame_ref.rax,
            frame_ref.rdi,
            frame_ref.rsi,
            frame_ref.rdx,
            frame_ref.r10,
            frame_ref.r8,
            frame_ref.vector,
            frame_ref.cs,
        )
    };
    if cs & 3 != 3 || vector != SYSCALL_VECTOR as u64 {
        serial::emergency_println(format_args!(
            "[FAIL] Invalid syscall entry frame: vector={} cs={:#x}",
            vector, cs
        ));
        unsafe { (*frame).rax = encode_error(ERROR_INVALID_ARGUMENT) };
        return frame;
    }

    match number {
        SYS_EXIT => process::exit_current(frame, a1),
        SYS_WRITE => {
            let result = syscall_write(a1 as Handle, a2, a3);
            unsafe { (*frame).rax = result };
            if (result as i64) >= 0 {
                process::acknowledge_displayd_recovery_write();
            }
            frame
        }
        SYS_CHANNEL_SEND => {
            unsafe {
                (*frame).rax = syscall_channel_send(a1 as Handle, a2, a3, a4 as Handle, a5 as u32)
            };
            frame
        }
        SYS_CHANNEL_RECEIVE => syscall_channel_receive(frame, a1 as Handle, a2, a3, a4),
        SYS_GETPID => {
            let pid = process::current_pid();
            unsafe { (*frame).rax = pid };
            serial::println(format_args!("[SYSC] getpid -> {}", pid));
            frame
        }
        SYS_YIELD => process::yield_current(frame),
        SYS_SYSTEM_QUERY => {
            unsafe { (*frame).rax = syscall_system_query(a1) };
            frame
        }
        SYS_DISPLAY_QUERY => {
            display::log_primary();
            unsafe { (*frame).rax = display::packed_primary_mode() };
            frame
        }
        SYS_GPU_QUERY => {
            let status = gpu_runtime::packed_status();
            serial::println(format_args!("[SHELL] gpu status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_GPU_PRESENT => {
            let authorized = matches!(
                process::current_lookup(a1 as Handle, RIGHT_WRITE),
                Ok(HandleObject::Device { object_id: crate::handles::GRAPHICS_PRESENT_OBJECT_ID })
            );
            let result = if !authorized {
                encode_error(ERROR_ACCESS_DENIED)
            } else {
                match gpu_runtime::present_from_displayd(a2) {
                    Ok(fence) => fence,
                    Err(error) => {
                        serial::println(format_args!("[FBCK] DISPLAYD accelerated present failed: {error}; GOP fallback retained"));
                        encode_error(ERROR_PROCESS_FAULT)
                    }
                }
            };
            unsafe { (*frame).rax = result };
            frame
        }
        SYS_NATIVE_GPU_QUERY => {
            let status = native_gpu::packed_status();
            serial::println(format_args!("[SHELL] native gpu status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_BINDING_QUERY => {
            let status = native_gpu_binding::packed_status();
            serial::println(format_args!("[SHELL] native gpu binding status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C2_QUERY => {
            let status = native_gpu_c2::packed_status();
            serial::println(format_args!("[SHELL] native gpu C2 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C3_QUERY => {
            let status = native_gpu_c3::packed_status();
            serial::println(format_args!("[SHELL] native gpu C3 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C4_QUERY => {
            let status = native_gpu_c4::packed_status();
            serial::println(format_args!("[SHELL] native gpu C4 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C5_QUERY => {
            let status = native_gpu_c5::packed_status();
            serial::println(format_args!("[SHELL] native gpu C5 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C6_QUERY => {
            let status = native_gpu_c6::packed_status();
            serial::println(format_args!("[SHELL] native gpu C6 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C7_QUERY => {
            let status = native_gpu_c7::packed_status();
            serial::println(format_args!("[SHELL] native gpu C7 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C8_QUERY => {
            let status = native_gpu_c8::packed_status();
            serial::println(format_args!("[SHELL] native gpu C8 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C9_QUERY => {
            let status = native_gpu_c9::packed_status();
            serial::println(format_args!("[SHELL] native gpu C9 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C10_QUERY => {
            let status = native_gpu_c10::packed_status();
            serial::println(format_args!("[SHELL] native gpu C10 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C11_QUERY => {
            let status = native_gpu_c11::packed_status();
            serial::println(format_args!("[SHELL] native gpu C11 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C12_QUERY => {
            let status = native_gpu_c12::packed_status();
            serial::println(format_args!("[SHELL] native gpu C12 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C13_QUERY => {
            let status = native_gpu_c13::packed_status();
            serial::println(format_args!("[SHELL] native gpu C13 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C14_QUERY => {
            let status = native_gpu_c14::packed_status();
            serial::println(format_args!("[SHELL] native gpu C14 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C15_QUERY => {
            let status = native_gpu_c15::packed_status();
            serial::println(format_args!("[SHELL] native gpu C15 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C16_QUERY => {
            let status = native_gpu_c16::packed_status();
            serial::println(format_args!("[SHELL] native gpu C16 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C17_QUERY => {
            let status = native_gpu_c17::packed_status();
            serial::println(format_args!("[SHELL] native gpu C17 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C18_QUERY => {
            let status = native_gpu_c18::packed_status();
            serial::println(format_args!("[SHELL] native gpu C18 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C19_QUERY => {
            let status = native_gpu_c19::packed_status();
            serial::println(format_args!("[SHELL] native gpu C19 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C20_QUERY => {
            let status = native_gpu_c20::packed_status();
            serial::println(format_args!("[SHELL] native gpu C20 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C21_QUERY => {
            let status = native_gpu_c21::packed_status();
            serial::println(format_args!("[SHELL] native gpu C21 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C22_QUERY => {
            let status = native_gpu_c22::packed_status();
            serial::println(format_args!("[SHELL] native gpu C22 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C23_QUERY => {
            let status = native_gpu_c23::packed_status();
            serial::println(format_args!("[SHELL] native gpu C23 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C24_QUERY => {
            let status = native_gpu_c24::packed_status();
            serial::println(format_args!("[SHELL] native gpu C24 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C25_QUERY => {
            let status = native_gpu_c25::packed_status();
            serial::println(format_args!("[SHELL] native gpu C25 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C26_QUERY => {
            let status = native_gpu_c26::packed_status();
            serial::println(format_args!("[SHELL] native gpu C26 status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C27_QUERY => {
            let status = native_gpu_c27::packed_status();
            serial::println(format_args!("[SHELL] native gpu C27 driver-core status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C28_QUERY => {
            let status = native_gpu_c28::packed_status();
            serial::println(format_args!("[SHELL] native gpu C28 memory/firmware/recovery status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C29_QUERY => {
            let status = native_gpu_c29::packed_status();
            serial::println(format_args!("[SHELL] native gpu C29 rings/queues/fences/DMA status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C30_QUERY => {
            let status = native_gpu_c30::packed_status();
            serial::println(format_args!("[SHELL] native gpu C30 complete basic display status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_NATIVE_GPU_C31_QUERY => {
            let status = native_gpu_c31::packed_status();
            serial::println(format_args!("[SHELL] native gpu C31 graphics+compute execution status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_GPU_RECOVER => {
            let authorized = matches!(
                process::current_lookup(a1 as Handle, RIGHT_WRITE),
                Ok(HandleObject::Device { object_id: crate::handles::GRAPHICS_PRESENT_OBJECT_ID })
            );
            let result = if !authorized {
                encode_error(ERROR_ACCESS_DENIED)
            } else {
                match gpu_runtime::recover_from_displayd(a2) {
                    Ok(fence) => {
                        process::note_displayd_recovery_result(true);
                        fence
                    }
                    Err(error) => {
                        process::note_displayd_recovery_result(false);
                        serial::println(format_args!("[FBCK] DISPLAYD GPU recovery failed: {error}; GOP fallback retained"));
                        encode_error(ERROR_PROCESS_FAULT)
                    }
                }
            };
            unsafe { (*frame).rax = result };
            frame
        }
        _ => {
            unsafe { (*frame).rax = encode_error(ERROR_INVALID_ARGUMENT) };
            frame
        }
    }
}

fn syscall_write(handle: Handle, address: u64, length: u64) -> u64 {
    let Ok(length) = usize::try_from(length) else {
        return encode_error(ERROR_INVALID_ARGUMENT);
    };
    if length == 0 || length > MAX_MESSAGE_BYTES {
        return encode_error(ERROR_INVALID_ARGUMENT);
    }
    match process::current_lookup(handle, RIGHT_WRITE) {
        Ok(HandleObject::Console) => {}
        Ok(_) => return encode_error(ERROR_BAD_HANDLE),
        Err(error) => return encode_error(process::map_handle_error(error)),
    }

    let mut bytes = [0u8; MAX_MESSAGE_BYTES];
    if process::current_copy_from_user(address, &mut bytes[..length]).is_err() {
        return encode_error(ERROR_ACCESS_DENIED);
    }
    serial::write_user_bytes(&bytes[..length]);
    length as u64
}

fn syscall_channel_send(
    handle: Handle,
    address: u64,
    length: u64,
    transferred_handle: Handle,
    transferred_rights: u32,
) -> u64 {
    let Ok(length) = usize::try_from(length) else {
        return encode_error(ERROR_INVALID_ARGUMENT);
    };
    if length == 0 || length > MAX_MESSAGE_BYTES {
        return encode_error(ERROR_INVALID_ARGUMENT);
    }
    let endpoint_side = match process::current_lookup(handle, RIGHT_WRITE) {
        Ok(HandleObject::ChannelEndpoint { channel: 0, side }) => side,
        Ok(_) => return encode_error(ERROR_BAD_HANDLE),
        Err(error) => return encode_error(process::map_handle_error(error)),
    };

    let capability = if transferred_handle == 0 {
        None
    } else {
        match process::current_transferable(transferred_handle, transferred_rights) {
            Ok(capability) => Some(capability),
            Err(error) => return encode_error(process::map_handle_error(error)),
        }
    };

    let mut bytes = [0u8; MAX_MESSAGE_BYTES];
    if process::current_copy_from_user(address, &mut bytes[..length]).is_err() {
        return encode_error(ERROR_ACCESS_DENIED);
    }
    match process::send_channel(endpoint_side, &bytes[..length], capability) {
        Ok(()) => {
            serial::println(format_args!(
                "[IPC ] pid={} sent {} bytes{}",
                process::current_pid(),
                length,
                if capability.is_some() { " with capability" } else { "" }
            ));
            length as u64
        }
        Err(_) => encode_error(ERROR_WOULD_BLOCK),
    }
}

fn syscall_channel_receive(
    frame: *mut UserTrapFrame,
    handle: Handle,
    address: u64,
    capacity: u64,
    out_handle_address: u64,
) -> *mut UserTrapFrame {
    let Ok(capacity) = usize::try_from(capacity) else {
        unsafe { (*frame).rax = encode_error(ERROR_INVALID_ARGUMENT) };
        return frame;
    };
    if capacity == 0 || capacity > MAX_MESSAGE_BYTES {
        unsafe { (*frame).rax = encode_error(ERROR_INVALID_ARGUMENT) };
        return frame;
    }
    let endpoint_side = match process::current_lookup(handle, RIGHT_READ) {
        Ok(HandleObject::ChannelEndpoint { channel: 0, side }) => side,
        Ok(_) => {
            unsafe { (*frame).rax = encode_error(ERROR_BAD_HANDLE) };
            return frame;
        }
        Err(error) => {
            unsafe { (*frame).rax = encode_error(process::map_handle_error(error)) };
            return frame;
        }
    };
    process::receive_or_block(frame, endpoint_side, address, capacity, out_handle_address)
}

fn syscall_system_query(query: u64) -> u64 {
    match query {
        0 => {
            serial::println(format_args!("[SHELL] dir C:\\SYSTEM\\SERVICES"));
            match vfs::log_directory(b"C:\\SYSTEM\\SERVICES") {
                Ok(count) => count as u64,
                Err(_) => encode_error(ERROR_INVALID_ARGUMENT),
            }
        }
        1 => {
            serial::println(format_args!("[SHELL] ps"));
            process::log_processes();
            0
        }
        2 => {
            serial::println(format_args!("[SHELL] services"));
            namespace::log_services();
            0
        }
        3 => {
            let mut status = [0u8; 64];
            let count = shared_memory::status_bytes(&mut status);
            serial::println(format_args!(
                "[SHELL] shared boot status: {}",
                core::str::from_utf8(&status[..count]).unwrap_or("invalid")
            ));
            count as u64
        }
        4 => {
            let ticks = percpu::ticks(percpu::bsp_index());
            serial::println(format_args!(
                "[SHELL] uptime: {} scheduler ticks ({} ms nominal)",
                ticks,
                ticks.saturating_mul(10)
            ));
            ticks
        }
        _ => encode_error(ERROR_INVALID_ARGUMENT),
    }
}
