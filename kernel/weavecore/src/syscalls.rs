use crate::abi::*;
use crate::handles::{Handle, HandleObject, RIGHT_CLOSE, RIGHT_READ, RIGHT_WRITE};
use crate::ipc::MAX_MESSAGE_BYTES;
use crate::user::UserTrapFrame;
use titanweave_forgeaudio_abi::{
    AudioAbiInfo, AudioControlOp, AudioControlRequest, AudioControlResponse, AudioDeviceInfo,
    AudioEndpointInfo, AudioObjectKind, AudioStreamConfig, FORGEAUDIO_ABI_VERSION,
};
use crate::{display, forgeaudio, gpu_runtime, native_gpu, native_gpu_binding, native_gpu_c2, native_gpu_c3, native_gpu_c4, native_gpu_c5, native_gpu_c6, native_gpu_c7, native_gpu_c8, native_gpu_c9, native_gpu_c10, native_gpu_c11, native_gpu_c12, native_gpu_c13, native_gpu_c14, native_gpu_c15, native_gpu_c16, native_gpu_c17, native_gpu_c18, native_gpu_c19, native_gpu_c20, native_gpu_c21, native_gpu_c22, native_gpu_c23, native_gpu_c24, native_gpu_c25, native_gpu_c26, native_gpu_c27, native_gpu_c28, native_gpu_c29, native_gpu_c30, native_gpu_c31, native_gpu_c32, namespace, percpu, process, serial, shared_memory, vfs};

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
        SYS_NATIVE_GPU_C32_QUERY => {
            let status = native_gpu_c32::packed_status();
            serial::println(format_args!("[SHELL] native gpu C32 production/stability final status: packed={:#x}", status));
            unsafe { (*frame).rax = status };
            frame
        }
        SYS_AUDIO_ABI_QUERY => {
            unsafe { (*frame).rax = syscall_audio_abi_query(a1, a2) };
            frame
        }
        SYS_AUDIO_ENUMERATE => {
            unsafe { (*frame).rax = syscall_audio_enumerate(a1, a2, a3, a4, a5) };
            frame
        }
        SYS_AUDIO_CONTROL => {
            unsafe { (*frame).rax = syscall_audio_control(a1, a2, a3, a4, a5) };
            frame
        }
        SYS_AUDIO_SERVER_CONTROL => {
            unsafe { (*frame).rax = syscall_audio_server_control(a1, a2, a3, a4, a5) };
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



fn syscall_audio_server_control(
    operation: u64,
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
) -> u64 {
    const REGISTER: u64 = 1;
    const PUBLISH: u64 = 2;
    const HEARTBEAT: u64 = 3;

    match operation {
        REGISTER => {
            if arg0 != 0 || arg1 != 0 || arg2 != 0 || arg3 != 0 {
                return encode_error(ERROR_INVALID_ARGUMENT);
            }
            match process::register_forgeaudiod() {
                Ok(pid) => pid,
                Err("ForgeAudioD singleton already registered") => encode_error(ERROR_BUSY),
                Err(_) => encode_error(ERROR_ACCESS_DENIED),
            }
        }
        PUBLISH => {
            let device_handle = arg0 as Handle;
            let Ok(route_count) = u32::try_from(arg1) else {
                return encode_error(ERROR_INVALID_ARGUMENT);
            };
            let Ok(graph_generation) = u32::try_from(arg2) else {
                return encode_error(ERROR_INVALID_ARGUMENT);
            };
            let Ok(recovery_count) = u32::try_from(arg3) else {
                return encode_error(ERROR_INVALID_ARGUMENT);
            };
            if route_count != 2 || graph_generation == 0 || recovery_count == 0 {
                return encode_error(ERROR_INVALID_ARGUMENT);
            }
            let device_object_id = match process::current_lookup(device_handle, RIGHT_READ) {
                Ok(HandleObject::Audio { object_id, kind: AudioObjectKind::Device }) => object_id,
                Ok(_) => return encode_error(ERROR_BAD_HANDLE),
                Err(error) => return encode_error(process::map_handle_error(error)),
            };
            let Some(snapshot) = forgeaudio::server_ownership_snapshot(
                process::current_pid(),
                device_object_id,
            ) else {
                return encode_error(ERROR_NOT_FOUND);
            };
            if snapshot.streams < 2
                || snapshot.playback_streams == 0
                || snapshot.capture_streams == 0
                || snapshot.prepared_streams < 2
                || snapshot.buffers < 2
                || snapshot.clocks == 0
                || snapshot.events == 0
                || snapshot.fences == 0
            {
                return encode_error(ERROR_NOT_READY);
            }
            if process::note_forgeaudiod_ready(route_count, graph_generation, recovery_count).is_err() {
                return encode_error(ERROR_ACCESS_DENIED);
            }
            serial::println(format_args!(
                "[K15D] ForgeAudioD ownership verified: device={:#x} streams={} playback={} capture={} prepared={} buffers={} clocks={} events={} fences={} routes={} graph_generation={} recovery=true",
                device_object_id,
                snapshot.streams,
                snapshot.playback_streams,
                snapshot.capture_streams,
                snapshot.prepared_streams,
                snapshot.buffers,
                snapshot.clocks,
                snapshot.events,
                snapshot.fences,
                route_count,
                graph_generation,
            ));
            u64::from(snapshot.streams)
                | (u64::from(snapshot.buffers) << 8)
                | (u64::from(snapshot.clocks) << 16)
                | (u64::from(snapshot.events) << 24)
                | (u64::from(snapshot.fences) << 32)
        }
        HEARTBEAT => {
            if arg0 == 0 || arg1 != 0 || arg2 != 0 || arg3 != 0 {
                return encode_error(ERROR_INVALID_ARGUMENT);
            }
            match process::note_forgeaudiod_heartbeat(arg0) {
                Ok(()) => arg0,
                Err(_) => encode_error(ERROR_INVALID_STATE),
            }
        }
        _ => encode_error(ERROR_INVALID_ARGUMENT),
    }
}

fn syscall_audio_abi_query(output_address: u64, output_length: u64) -> u64 {
    if output_length != core::mem::size_of::<AudioAbiInfo>() as u64 {
        return encode_error(ERROR_BUFFER_TOO_SMALL);
    }
    let info = forgeaudio::abi_info();
    match copy_struct_to_user(output_address, &info) {
        Ok(()) => core::mem::size_of::<AudioAbiInfo>() as u64,
        Err(error) => encode_error(error),
    }
}

fn syscall_audio_enumerate(
    kind_raw: u64,
    parent_object_id: u64,
    index_raw: u64,
    output_address: u64,
    output_length: u64,
) -> u64 {
    let Ok(index) = usize::try_from(index_raw) else {
        return encode_error(ERROR_INVALID_ARGUMENT);
    };
    match kind_raw as u32 {
        value if value == AudioObjectKind::Device as u32 => {
            if output_length != core::mem::size_of::<AudioDeviceInfo>() as u64 {
                return encode_error(ERROR_BUFFER_TOO_SMALL);
            }
            let Some(info) = forgeaudio::enumerate_device(index) else {
                return encode_error(ERROR_NOT_FOUND);
            };
            match copy_struct_to_user(output_address, &info) {
                Ok(()) => core::mem::size_of::<AudioDeviceInfo>() as u64,
                Err(error) => encode_error(error),
            }
        }
        value if value == AudioObjectKind::Endpoint as u32 => {
            if parent_object_id == 0 {
                return encode_error(ERROR_INVALID_ARGUMENT);
            }
            if output_length != core::mem::size_of::<AudioEndpointInfo>() as u64 {
                return encode_error(ERROR_BUFFER_TOO_SMALL);
            }
            let Some(info) = forgeaudio::enumerate_endpoint(parent_object_id, index) else {
                return encode_error(ERROR_NOT_FOUND);
            };
            match copy_struct_to_user(output_address, &info) {
                Ok(()) => core::mem::size_of::<AudioEndpointInfo>() as u64,
                Err(error) => encode_error(error),
            }
        }
        _ => encode_error(ERROR_INVALID_ARGUMENT),
    }
}

fn syscall_audio_control(
    request_address: u64,
    request_length: u64,
    response_address: u64,
    response_length: u64,
    reserved: u64,
) -> u64 {
    if reserved != 0
        || request_length != core::mem::size_of::<AudioControlRequest>() as u64
        || response_length != core::mem::size_of::<AudioControlResponse>() as u64
    {
        return encode_error(ERROR_INVALID_ARGUMENT);
    }
    let request = match copy_struct_from_user::<AudioControlRequest>(request_address) {
        Ok(request) => request,
        Err(error) => return encode_error(error),
    };
    if request.abi_version != FORGEAUDIO_ABI_VERSION {
        return encode_error(ERROR_NOT_SUPPORTED);
    }
    let Some(operation) = AudioControlOp::from_raw(request.operation) else {
        return encode_error(ERROR_INVALID_ARGUMENT);
    };

    let response = match execute_audio_control(operation, request) {
        Ok(response) => response,
        Err(error) => return encode_error(error),
    };

    if let Err(error) = copy_struct_to_user(response_address, &response) {
        if creates_audio_handle(operation) && response.handle != 0 {
            cleanup_new_audio_handle(response.handle);
        }
        return encode_error(error);
    }
    core::mem::size_of::<AudioControlResponse>() as u64
}

fn execute_audio_control(
    operation: AudioControlOp,
    request: AudioControlRequest,
) -> Result<AudioControlResponse, i64> {
    match operation {
        AudioControlOp::OpenDevice => {
            if request.object_id == 0 {
                return Err(ERROR_INVALID_ARGUMENT);
            }
            let object = forgeaudio::open_device(request.object_id).map_err(map_audio_error)?;
            let handle = install_audio_handle(object, RIGHT_READ | RIGHT_WRITE | RIGHT_CLOSE)?;
            Ok(audio_response_for_object(handle, object, 0))
        }
        AudioControlOp::OpenStream => {
            let device_object_id = match process::current_lookup(request.handle, RIGHT_WRITE) {
                Ok(HandleObject::Audio { object_id, kind: AudioObjectKind::Device }) => object_id,
                Ok(_) => return Err(ERROR_BAD_HANDLE),
                Err(error) => return Err(process::map_handle_error(error)),
            };
            let object = forgeaudio::open_stream(
                process::current_pid(),
                device_object_id,
                request.object_id,
            )
            .map_err(map_audio_error)?;
            let handle = install_audio_handle(object, RIGHT_READ | RIGHT_WRITE | RIGHT_CLOSE)?;
            Ok(audio_response_for_object(
                handle,
                object,
                titanweave_forgeaudio_abi::AudioStreamState::Created as u32,
            ))
        }
        AudioControlOp::ConfigureStream => {
            let object_id = lookup_audio_handle(request.handle, RIGHT_WRITE, AudioObjectKind::Stream)?;
            let direction = u32::try_from(request.object_id).map_err(|_| ERROR_INVALID_ARGUMENT)?;
            let sample_format = u32::try_from(request.arg0).map_err(|_| ERROR_INVALID_ARGUMENT)?;
            let sample_rate_hz = u32::try_from(request.arg1).map_err(|_| ERROR_INVALID_ARGUMENT)?;
            let channels = u16::try_from(request.arg2).map_err(|_| ERROR_INVALID_ARGUMENT)?;
            let period_frames = request.arg3 as u32;
            let buffer_frames = (request.arg3 >> 32) as u32;
            let config = AudioStreamConfig {
                abi_version: FORGEAUDIO_ABI_VERSION,
                flags: request.flags,
                direction,
                sample_format,
                sample_rate_hz,
                channels,
                reserved0: 0,
                period_frames,
                buffer_frames,
                reserved1: 0,
            };
            forgeaudio::configure_stream(object_id, config).map_err(map_audio_error)?;
            let object = audio_object_ref(AudioObjectKind::Stream, object_id)?;
            Ok(audio_response_for_object(
                request.handle,
                object,
                titanweave_forgeaudio_abi::AudioStreamState::Configured as u32,
            ))
        }
        AudioControlOp::PrepareStream => stream_transition_response(
            request.handle,
            titanweave_forgeaudio_abi::AudioStreamState::Prepared,
            forgeaudio::prepare_stream,
        ),
        AudioControlOp::StartStream => stream_transition_response(
            request.handle,
            titanweave_forgeaudio_abi::AudioStreamState::Running,
            forgeaudio::start_stream,
        ),
        AudioControlOp::StopStream => stream_transition_response(
            request.handle,
            titanweave_forgeaudio_abi::AudioStreamState::Stopped,
            forgeaudio::stop_stream,
        ),
        AudioControlOp::DrainStream => stream_transition_response(
            request.handle,
            titanweave_forgeaudio_abi::AudioStreamState::Draining,
            forgeaudio::drain_stream,
        ),
        AudioControlOp::RecoverStream => stream_transition_response(
            request.handle,
            titanweave_forgeaudio_abi::AudioStreamState::Configured,
            forgeaudio::recover_stream,
        ),
        AudioControlOp::QueryPosition => {
            let object = process::current_lookup(request.handle, RIGHT_READ)
                .map_err(process::map_handle_error)?;
            match object {
                HandleObject::Audio { object_id, kind: AudioObjectKind::Stream } => {
                    let position = forgeaudio::stream_position(object_id).ok_or(ERROR_NOT_FOUND)?;
                    let object = audio_object_ref(AudioObjectKind::Stream, object_id)?;
                    let mut response = audio_response_for_object(
                        request.handle,
                        object,
                        position.state as u32,
                    );
                    response.value0 = position.frame_position;
                    Ok(response)
                }
                HandleObject::Audio { object_id, kind: AudioObjectKind::Clock } => {
                    let snapshot = forgeaudio::clock_snapshot(object_id).ok_or(ERROR_NOT_FOUND)?;
                    let object = audio_object_ref(AudioObjectKind::Clock, object_id)?;
                    let mut response = audio_response_for_object(request.handle, object, 0);
                    response.flags = snapshot.flags;
                    response.value0 = snapshot.tick;
                    response.value1 = snapshot.nanoseconds;
                    response.value2 = snapshot.frame_position;
                    response.value3 =
                        u64::from(snapshot.rate_numerator) | (u64::from(snapshot.rate_denominator) << 32);
                    Ok(response)
                }
                _ => Err(ERROR_BAD_HANDLE),
            }
        }
        AudioControlOp::CreateBuffer => {
            let byte_capacity = u32::try_from(request.arg0).map_err(|_| ERROR_INVALID_ARGUMENT)?;
            let frame_stride = u32::try_from(request.arg1).map_err(|_| ERROR_INVALID_ARGUMENT)?;
            let object = forgeaudio::create_buffer(
                process::current_pid(),
                byte_capacity,
                frame_stride,
                request.flags,
            )
            .map_err(map_audio_error)?;
            let handle = install_audio_handle(object, RIGHT_READ | RIGHT_WRITE | RIGHT_CLOSE)?;
            let info = forgeaudio::buffer_info(object.object_id).ok_or(ERROR_PROCESS_FAULT)?;
            let mut response = audio_response_for_object(handle, object, 0);
            response.flags = info.flags;
            response.value0 = u64::from(info.byte_capacity);
            response.value1 = u64::from(info.frame_stride_bytes);
            response.value2 = u64::from(info.frame_capacity);
            Ok(response)
        }
        AudioControlOp::CreateClock => {
            let rate_numerator = u32::try_from(request.arg0).map_err(|_| ERROR_INVALID_ARGUMENT)?;
            let rate_denominator = u32::try_from(request.arg1).map_err(|_| ERROR_INVALID_ARGUMENT)?;
            let object = forgeaudio::create_clock(process::current_pid(), rate_numerator, rate_denominator)
                .map_err(map_audio_error)?;
            let handle = install_audio_handle(object, RIGHT_READ | RIGHT_CLOSE)?;
            Ok(audio_response_for_object(handle, object, 0))
        }
        AudioControlOp::CreateEvent => {
            let object = forgeaudio::create_event(process::current_pid()).map_err(map_audio_error)?;
            let handle = install_audio_handle(object, RIGHT_READ | RIGHT_CLOSE)?;
            Ok(audio_response_for_object(handle, object, 0))
        }
        AudioControlOp::CreateFence => {
            let object = forgeaudio::create_fence(process::current_pid(), request.arg0)
                .map_err(map_audio_error)?;
            let handle = install_audio_handle(object, RIGHT_READ | RIGHT_WRITE | RIGHT_CLOSE)?;
            Ok(audio_response_for_object(handle, object, 0))
        }
        AudioControlOp::PollEvent => {
            let object_id = lookup_audio_handle(request.handle, RIGHT_READ, AudioObjectKind::Event)?;
            let Some(record) = forgeaudio::poll_event(object_id).map_err(map_audio_error)? else {
                return Err(ERROR_WOULD_BLOCK);
            };
            let object = audio_object_ref(AudioObjectKind::Event, object_id)?;
            let mut response = audio_response_for_object(request.handle, object, record.kind);
            response.flags = record.code;
            response.object_id = record.object_id;
            response.value0 = record.sequence;
            response.value1 = record.timestamp_tick;
            response.value2 = record.value0;
            response.value3 = record.value1;
            Ok(response)
        }
        AudioControlOp::QueryFence => {
            let object_id = lookup_audio_handle(request.handle, RIGHT_READ, AudioObjectKind::Fence)?;
            let info = forgeaudio::fence_info(object_id).ok_or(ERROR_NOT_FOUND)?;
            let object = audio_object_ref(AudioObjectKind::Fence, object_id)?;
            let mut response = audio_response_for_object(request.handle, object, 0);
            response.flags = info.flags;
            response.value0 = info.target_value;
            response.value1 = info.completed_value;
            response.value2 = info.sequence;
            Ok(response)
        }
        AudioControlOp::CloseObject => {
            let object = process::current_lookup(request.handle, RIGHT_CLOSE)
                .map_err(process::map_handle_error)?;
            let HandleObject::Audio { object_id, kind } = object else {
                return Err(ERROR_BAD_HANDLE);
            };
            forgeaudio::release_object(kind, object_id, process::current_pid()).map_err(map_audio_error)?;
            process::current_close_handle(request.handle).map_err(process::map_handle_error)?;
            Ok(AudioControlResponse {
                abi_version: FORGEAUDIO_ABI_VERSION,
                object_kind: kind as u32,
                handle: 0,
                state: titanweave_forgeaudio_abi::AudioStreamState::Closed as u32,
                object_id,
                generation: 0,
                flags: 0,
                value0: 0,
                value1: 0,
                value2: 0,
                value3: 0,
            })
        }
    }
}

fn stream_transition_response(
    handle: Handle,
    expected_state: titanweave_forgeaudio_abi::AudioStreamState,
    operation: fn(u64) -> Result<(), &'static str>,
) -> Result<AudioControlResponse, i64> {
    let object_id = lookup_audio_handle(handle, RIGHT_WRITE, AudioObjectKind::Stream)?;
    operation(object_id).map_err(map_audio_error)?;
    let position = forgeaudio::stream_position(object_id).ok_or(ERROR_NOT_FOUND)?;
    if position.state != expected_state {
        return Err(ERROR_PROCESS_FAULT);
    }
    let object = audio_object_ref(AudioObjectKind::Stream, object_id)?;
    let mut response = audio_response_for_object(handle, object, position.state as u32);
    response.value0 = position.frame_position;
    Ok(response)
}

fn install_audio_handle(
    object: forgeaudio::AudioObjectRef,
    rights: u32,
) -> Result<Handle, i64> {
    match process::current_allocate_handle(
        HandleObject::Audio {
            object_id: object.object_id,
            kind: object.kind,
        },
        rights,
    ) {
        Ok(handle) => Ok(handle),
        Err(error) => {
            let _ = forgeaudio::release_object(object.kind, object.object_id, process::current_pid());
            Err(process::map_handle_error(error))
        }
    }
}

fn lookup_audio_handle(
    handle: Handle,
    rights: u32,
    expected_kind: AudioObjectKind,
) -> Result<u64, i64> {
    match process::current_lookup(handle, rights) {
        Ok(HandleObject::Audio { object_id, kind }) if kind == expected_kind => Ok(object_id),
        Ok(_) => Err(ERROR_BAD_HANDLE),
        Err(error) => Err(process::map_handle_error(error)),
    }
}

fn audio_object_ref(kind: AudioObjectKind, object_id: u64) -> Result<forgeaudio::AudioObjectRef, i64> {
    let generation = forgeaudio::object_generation(kind, object_id).ok_or(ERROR_NOT_FOUND)?;
    Ok(forgeaudio::AudioObjectRef {
        kind,
        object_id,
        generation,
    })
}

fn audio_response_for_object(
    handle: Handle,
    object: forgeaudio::AudioObjectRef,
    state: u32,
) -> AudioControlResponse {
    AudioControlResponse {
        abi_version: FORGEAUDIO_ABI_VERSION,
        object_kind: object.kind as u32,
        handle,
        state,
        object_id: object.object_id,
        generation: object.generation,
        flags: 0,
        value0: 0,
        value1: 0,
        value2: 0,
        value3: 0,
    }
}

fn creates_audio_handle(operation: AudioControlOp) -> bool {
    matches!(
        operation,
        AudioControlOp::OpenDevice
            | AudioControlOp::OpenStream
            | AudioControlOp::CreateBuffer
            | AudioControlOp::CreateClock
            | AudioControlOp::CreateEvent
            | AudioControlOp::CreateFence
    )
}

fn cleanup_new_audio_handle(handle: Handle) {
    let Ok(HandleObject::Audio { object_id, kind }) = process::current_lookup(handle, RIGHT_CLOSE) else {
        return;
    };
    let _ = forgeaudio::release_object(kind, object_id, process::current_pid());
    let _ = process::current_close_handle(handle);
}

fn map_audio_error(error: &'static str) -> i64 {
    match error {
        "audio device not found"
        | "audio stream parent device not found"
        | "audio stream endpoint does not belong to device"
        | "audio stream not found"
        | "audio stream endpoint disappeared"
        | "audio buffer not found"
        | "ForgeAudio event object not found"
        | "ForgeAudio fence not found" => ERROR_NOT_FOUND,
        "ForgeAudio device table is full"
        | "ForgeAudio endpoint table is full"
        | "ForgeAudio stream table is full"
        | "ForgeAudio buffer table is full"
        | "ForgeAudio clock table is full"
        | "ForgeAudio event table is full"
        | "ForgeAudio fence table is full"
        | "ForgeAudio event queue is full" => ERROR_NO_SPACE,
        "stream configuration is invalid in current state"
        | "stream must be configured before prepare"
        | "stream must be prepared before start"
        | "only a running stream can drain"
        | "stream cannot stop from current state"
        | "only a configured faulted stream can recover"
        | "audio stream position advances only while active" => ERROR_INVALID_STATE,
        "unsupported ForgeAudio ABI version" => ERROR_NOT_SUPPORTED,
        _ => ERROR_INVALID_ARGUMENT,
    }
}

fn copy_struct_from_user<T: Copy>(address: u64) -> Result<T, i64> {
    if address == 0 {
        return Err(ERROR_INVALID_ARGUMENT);
    }
    let mut value = core::mem::MaybeUninit::<T>::uninit();
    let bytes = unsafe {
        core::slice::from_raw_parts_mut(
            value.as_mut_ptr().cast::<u8>(),
            core::mem::size_of::<T>(),
        )
    };
    process::current_copy_from_user(address, bytes).map_err(|_| ERROR_ACCESS_DENIED)?;
    Ok(unsafe { value.assume_init() })
}

fn copy_struct_to_user<T: Copy>(address: u64, value: &T) -> Result<(), i64> {
    if address == 0 {
        return Err(ERROR_INVALID_ARGUMENT);
    }
    let bytes = unsafe {
        core::slice::from_raw_parts(
            (value as *const T).cast::<u8>(),
            core::mem::size_of::<T>(),
        )
    };
    process::current_copy_to_user(address, bytes).map_err(|_| ERROR_ACCESS_DENIED)
}
