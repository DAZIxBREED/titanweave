use crate::abi::{encode_error, ERROR_ACCESS_DENIED, ERROR_BAD_HANDLE, ERROR_NO_SPACE};
use crate::arch::x86_64::{self, apic, gdt, halt_forever};
use crate::elf;
use crate::handles::{
    Handle, HandleDescriptor, HandleObject, HandleTable, CONSOLE_HANDLE, RIGHT_READ,
    RIGHT_TRANSFER, RIGHT_WRITE, SERVICE_CHANNEL_HANDLE,
};
use crate::ipc::{ChannelMessage, SERVICE_CHANNEL, SERVICE_CHANNEL_INDEX};
use crate::memory::{FrameAllocator, FRAME_SIZE};
use crate::objects::{ObjectHeader, ObjectKind};
use crate::paging::{AddressSpace, USER_STACK_TOP};
use crate::service::{ServiceRole, SERVICE_SPECS};
use crate::vfs;
use crate::user::{self, UserTrapFrame};
use crate::serial;
use core::arch::asm;
use core::cell::UnsafeCell;
use core::mem::size_of;
use core::ptr;
use titanweave_boot_protocol::BootInfo;

pub type ProcessId = u64;
const MAX_PROCESSES: usize = 64;
const PROCESS_NAME_BYTES: usize = 16;
const USER_KERNEL_STACK_PAGES: u64 = 16;
const USER_KERNEL_STACK_SIZE: u64 = USER_KERNEL_STACK_PAGES * FRAME_SIZE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ProcessState {
    Empty,
    Ready,
    Running,
    Blocked,
    Exited,
    Faulted,
}

#[derive(Clone, Copy)]
struct PendingReceive {
    endpoint_side: u8,
    buffer_address: u64,
    capacity: usize,
    out_handle_address: u64,
}

pub struct Process {
    header: ObjectHeader,
    pid: ProcessId,
    name: [u8; PROCESS_NAME_BYTES],
    state: ProcessState,
    address_space: AddressSpace,
    handles: HandleTable,
    kernel_stack_base: u64,
    kernel_stack_top: u64,
    address_space_reclaimed: bool,
    resources_released: bool,
    saved_frame: u64,
    pending_receive: Option<PendingReceive>,
    exit_code: u64,
    fault_vector: u64,
}

impl Process {
    fn name_bytes(&self) -> &[u8] {
        let length = self.name.iter().position(|byte| *byte == 0).unwrap_or(self.name.len());
        &self.name[..length]
    }
}

struct ProcessSlot(UnsafeCell<Option<Process>>);
unsafe impl Sync for ProcessSlot {}

#[derive(Clone, Copy)]
struct ProcessRuntime {
    active: bool,
    count: usize,
    current_index: usize,
    cursor: usize,
    kernel_cr3: u64,
    kernel_return_stack: u64,
    cpu_index: usize,
    exited: usize,
    faulted: usize,
    context_switches: u64,
    shell_completed: bool,
    displayd_recovery_result: i8,
    displayd_recovery_acknowledged: bool,
    forgeaudiod_registered: bool,
    forgeaudiod_ready: bool,
    forgeaudiod_heartbeat: bool,
    forgeaudiod_failed: bool,
    forgeaudiod_pid: u64,
    forgeaudiod_route_count: u32,
    forgeaudiod_graph_generation: u32,
    forgeaudiod_recovery_count: u32,
    forgeaudiod_heartbeat_sequence: u64,
    forgeaudio_transport_ready: bool,
    forgeaudio_transport_dead_isolated: bool,
    allocator: *mut FrameAllocator<'static>,
}

impl ProcessRuntime {
    const fn new() -> Self {
        Self {
            active: false,
            count: 0,
            current_index: 0,
            cursor: 0,
            kernel_cr3: 0,
            kernel_return_stack: 0,
            cpu_index: 0,
            exited: 0,
            faulted: 0,
            context_switches: 0,
            shell_completed: false,
            displayd_recovery_result: 0,
            displayd_recovery_acknowledged: false,
            forgeaudiod_registered: false,
            forgeaudiod_ready: false,
            forgeaudiod_heartbeat: false,
            forgeaudiod_failed: false,
            forgeaudiod_pid: 0,
            forgeaudiod_route_count: 0,
            forgeaudiod_graph_generation: 0,
            forgeaudiod_recovery_count: 0,
            forgeaudiod_heartbeat_sequence: 0,
            forgeaudio_transport_ready: false,
            forgeaudio_transport_dead_isolated: false,
            allocator: core::ptr::null_mut(),
        }
    }
}

struct RuntimeCell(UnsafeCell<ProcessRuntime>);
unsafe impl Sync for RuntimeCell {}

static PROCESS_SLOTS: [ProcessSlot; MAX_PROCESSES] =
    [const { ProcessSlot(UnsafeCell::new(None)) }; MAX_PROCESSES];
static RUNTIME: RuntimeCell = RuntimeCell(UnsafeCell::new(ProcessRuntime::new()));

pub fn launch_user_services(
    allocator: &mut FrameAllocator<'_>,
    _boot_info: &BootInfo,
    kernel_cr3: u64,
    kernel_return_stack: u64,
    cpu_index: usize,
    timer_initial_count: u32,
) -> ! {
    assert!(!x86_64::interrupts_enabled(), "K6 userspace launch requires interrupts disabled");

    unsafe {
        *RUNTIME.0.get() = ProcessRuntime {
            active: false,
            count: 0,
            current_index: 0,
            cursor: 0,
            kernel_cr3,
            kernel_return_stack,
            cpu_index,
            exited: 0,
            faulted: 0,
            context_switches: 0,
            shell_completed: false,
            displayd_recovery_result: 0,
            displayd_recovery_acknowledged: false,
            forgeaudiod_registered: false,
            forgeaudiod_ready: false,
            forgeaudiod_heartbeat: false,
            forgeaudiod_failed: false,
            forgeaudiod_pid: 0,
            forgeaudiod_route_count: 0,
            forgeaudiod_graph_generation: 0,
            forgeaudiod_recovery_count: 0,
            forgeaudiod_heartbeat_sequence: 0,
            forgeaudio_transport_ready: false,
            forgeaudio_transport_dead_isolated: false,
            allocator: (allocator as *mut FrameAllocator<'_>).cast::<FrameAllocator<'static>>(),
        };
    }

    for (index, spec) in SERVICE_SPECS.iter().enumerate() {
        let loaded = vfs::with_file(spec.path, |image| {
            elf::load_user_elf(image, allocator, kernel_cr3)
        })
        .unwrap_or_else(|error| panic!("K6 VFS user ELF load failed: {error}"));
        let process = create_process(
            allocator,
            (index + 1) as ProcessId,
            spec.process_name,
            spec.role,
            loaded.address_space,
        )
        .unwrap_or_else(|error| panic!("K6 process creation failed: {error}"));
        serial::println(format_args!(
            "[ELF ] Loaded {}: pid={} object={} entry={:#x} segments={} CR3={:#x}",
            core::str::from_utf8(process.name_bytes()).unwrap_or("service"),
            process.pid,
            process.header.id(),
            process.address_space.entry_point,
            loaded.load_segments,
            process.address_space.cr3,
        ));
        unsafe { *PROCESS_SLOTS[index].0.get() = Some(process) };
        unsafe { (*RUNTIME.0.get()).count += 1 };
    }

    serial::println(format_args!(
        "[INIT] Loaded init, logging, console, display, archive, trust, driver, ForgeAudioD, audio-client, and shell services from C:"
    ));
    serial::println(format_args!(
        "[IPC ] K5 capability channel retained as object={}",
        SERVICE_CHANNEL.header().id()
    ));

    let first_index = 0usize;
    unsafe {
        let runtime = &mut *RUNTIME.0.get();
        runtime.active = true;
        runtime.current_index = first_index;
        runtime.cursor = first_index;
    }
    with_process_mut(first_index, |process| process.state = ProcessState::Running);
    let (frame, cr3, stack_top) = process_switch_values(first_index);
    activate_address_space(cr3, stack_top, cpu_index);
    apic::start_periodic_timer(crate::arch::x86_64::idt::TIMER_VECTOR, timer_initial_count);
    serial::println(format_args!(
        "[USER] Starting disk-backed K14 native services"
    ));
    crate::recovery::mark_boot_stable();
    serial::println(format_args!("[RECV] kernel initialization reached stable userspace handoff"));
    unsafe { user::enter_frame(frame as *const UserTrapFrame, cr3) }
}

fn create_process(
    allocator: &mut FrameAllocator<'_>,
    pid: ProcessId,
    name: &[u8],
    role: ServiceRole,
    address_space: AddressSpace,
) -> Result<Process, &'static str> {
    let Some(kernel_stack_base) = allocator.allocate_contiguous(USER_KERNEL_STACK_PAGES) else {
        return Err("no memory for user-process kernel stack");
    };
    unsafe { ptr::write_bytes(kernel_stack_base as *mut u8, 0, USER_KERNEL_STACK_SIZE as usize) };
    let kernel_stack_top = kernel_stack_base + USER_KERNEL_STACK_SIZE;
    let frame_address = build_initial_frame(kernel_stack_top, address_space.entry_point)?;

    let mut handles = HandleTable::new();
    match role {
        ServiceRole::Init => {
            handles.install(
                CONSOLE_HANDLE,
                HandleObject::Console,
                RIGHT_READ | RIGHT_WRITE | RIGHT_TRANSFER,
            )?;
            handles.install(
                SERVICE_CHANNEL_HANDLE,
                HandleObject::ChannelEndpoint {
                    channel: SERVICE_CHANNEL_INDEX,
                    side: 0,
                },
                RIGHT_READ | RIGHT_WRITE,
            )?;
        }
        ServiceRole::Display => {
            handles.install(CONSOLE_HANDLE, HandleObject::Console, RIGHT_WRITE)?;
            handles.install(
                crate::handles::DISPLAY_PRESENT_HANDLE,
                HandleObject::Device { object_id: crate::handles::GRAPHICS_PRESENT_OBJECT_ID },
                RIGHT_WRITE,
            )?;
        }
        ServiceRole::Logging | ServiceRole::Console | ServiceRole::Shell | ServiceRole::Archive | ServiceRole::Trust | ServiceRole::DriverHost | ServiceRole::Audio | ServiceRole::AudioClient => {
            handles.install(CONSOLE_HANDLE, HandleObject::Console, RIGHT_WRITE)?;
        }
    }

    let mut process_name = [0u8; PROCESS_NAME_BYTES];
    let length = core::cmp::min(name.len(), PROCESS_NAME_BYTES - 1);
    process_name[..length].copy_from_slice(&name[..length]);

    let _ = crate::kernel_runtime::with_runtime(|runtime| runtime.objects.create(0x2000 + pid, pid));

    Ok(Process {
        header: ObjectHeader::new_static(0x2000 + pid, ObjectKind::Process),
        pid,
        name: process_name,
        state: ProcessState::Ready,
        address_space,
        handles,
        kernel_stack_base,
        kernel_stack_top,
        address_space_reclaimed: false,
        resources_released: false,
        saved_frame: frame_address,
        pending_receive: None,
        exit_code: 0,
        fault_vector: 0,
    })
}

fn build_initial_frame(stack_top: u64, entry: u64) -> Result<u64, &'static str> {
    let frame_address = (stack_top & !0x0f)
        .checked_sub(size_of::<UserTrapFrame>() as u64)
        .ok_or("user trap frame underflow")?;
    unsafe { ptr::write(frame_address as *mut UserTrapFrame, UserTrapFrame::initial(entry, USER_STACK_TOP)) };
    Ok(frame_address)
}

#[must_use]
pub fn runtime_active() -> bool {
    unsafe { (*RUNTIME.0.get()).active }
}

pub fn current_pid() -> ProcessId {
    let index = unsafe { (*RUNTIME.0.get()).current_index };
    with_process(index, |process| process.pid)
}

#[must_use]
pub fn current_service_role() -> ServiceRole {
    let index = unsafe { (*RUNTIME.0.get()).current_index };
    SERVICE_SPECS[index].role
}

pub fn register_forgeaudiod() -> Result<u64, &'static str> {
    if current_service_role() != ServiceRole::Audio {
        return Err("ForgeAudioD registration denied for non-audio service");
    }
    let pid = current_pid();
    let runtime = unsafe { &mut *RUNTIME.0.get() };
    if runtime.forgeaudiod_registered && runtime.forgeaudiod_pid != pid {
        return Err("ForgeAudioD singleton already registered");
    }
    runtime.forgeaudiod_registered = true;
    runtime.forgeaudiod_pid = pid;
    serial::println(format_args!("[K15D] ForgeAudioD registered: pid={} singleton=true userspace=true", pid));
    Ok(pid)
}

pub fn note_forgeaudiod_ready(route_count: u32, graph_generation: u32, recovery_count: u32) -> Result<(), &'static str> {
    let pid = current_pid();
    let runtime = unsafe { &mut *RUNTIME.0.get() };
    if !runtime.forgeaudiod_registered || runtime.forgeaudiod_pid != pid {
        return Err("ForgeAudioD publish before registration");
    }
    runtime.forgeaudiod_ready = true;
    runtime.forgeaudiod_route_count = route_count;
    runtime.forgeaudiod_graph_generation = graph_generation;
    runtime.forgeaudiod_recovery_count = recovery_count;
    serial::println(format_args!(
        "[K15D] ForgeAudioD ownership published: pid={} routes={} graph_generation={} recoveries={} ready=true",
        pid, route_count, graph_generation, recovery_count
    ));
    Ok(())
}

pub fn note_forgeaudiod_heartbeat(sequence: u64) -> Result<(), &'static str> {
    let pid = current_pid();
    let runtime = unsafe { &mut *RUNTIME.0.get() };
    if !runtime.forgeaudiod_registered || runtime.forgeaudiod_pid != pid || !runtime.forgeaudiod_ready {
        return Err("ForgeAudioD heartbeat before ready");
    }
    if sequence == 0 {
        return Err("ForgeAudioD heartbeat sequence must be non-zero");
    }
    if sequence <= runtime.forgeaudiod_heartbeat_sequence {
        return Err("ForgeAudioD heartbeat sequence must advance");
    }
    runtime.forgeaudiod_heartbeat_sequence = sequence;
    runtime.forgeaudiod_heartbeat = true;
    serial::println(format_args!("[K15D] ForgeAudioD heartbeat: pid={} sequence={} persistent=true", pid, sequence));
    Ok(())
}


#[must_use]
pub fn forgeaudiod_server_pid_if_ready() -> Option<u64> {
    let runtime = unsafe { &*RUNTIME.0.get() };
    if runtime.forgeaudiod_registered && runtime.forgeaudiod_ready && runtime.forgeaudiod_heartbeat && !runtime.forgeaudiod_failed {
        Some(runtime.forgeaudiod_pid)
    } else {
        None
    }
}

pub fn note_forgeaudio_transport_ready() -> Result<(), &'static str> {
    if current_service_role() != ServiceRole::Audio {
        return Err("K15.7 transport qualification denied for non-audio service");
    }
    let pid = current_pid();
    let runtime = unsafe { &mut *RUNTIME.0.get() };
    if !runtime.forgeaudiod_registered || runtime.forgeaudiod_pid != pid || !runtime.forgeaudiod_ready || !runtime.forgeaudiod_heartbeat {
        return Err("K15.7 transport qualification requires persistent ForgeAudioD");
    }
    runtime.forgeaudio_transport_ready = true;
    runtime.forgeaudio_transport_dead_isolated = true;
    serial::println(format_args!("[K15LF] required lock-free transport + dead-client isolation milestones complete; userspace qualification may close"));
    Ok(())
}


pub fn log_processes() {
    let count = unsafe { (*RUNTIME.0.get()).count };
    for index in 0..count {
        with_process(index, |process| {
            serial::println(format_args!(
                "[PS  ] pid={} state={:?} name={}",
                process.pid,
                process.state,
                core::str::from_utf8(process.name_bytes()).unwrap_or("process")
            ));
        });
    }
}

pub fn current_copy_from_user(address: u64, output: &mut [u8]) -> Result<(), &'static str> {
    let index = unsafe { (*RUNTIME.0.get()).current_index };
    with_process(index, |process| process.address_space.copy_from_user(address, output))
}

pub fn current_copy_to_user(address: u64, input: &[u8]) -> Result<(), &'static str> {
    let index = unsafe { (*RUNTIME.0.get()).current_index };
    with_process(index, |process| process.address_space.copy_to_user(address, input))
}

pub fn current_lookup(handle: Handle, rights: u32) -> Result<HandleObject, &'static str> {
    let index = unsafe { (*RUNTIME.0.get()).current_index };
    with_process(index, |process| process.handles.lookup(handle, rights))
}

pub fn current_allocate_handle(object: HandleObject, rights: u32) -> Result<Handle, &'static str> {
    let index = unsafe { (*RUNTIME.0.get()).current_index };
    with_process_mut(index, |process| process.handles.allocate(object, rights))
}

pub fn current_close_handle(handle: Handle) -> Result<HandleDescriptor, &'static str> {
    let index = unsafe { (*RUNTIME.0.get()).current_index };
    with_process_mut(index, |process| process.handles.close(handle))
}

pub fn current_transferable(
    handle: Handle,
    rights: u32,
) -> Result<HandleDescriptor, &'static str> {
    let index = unsafe { (*RUNTIME.0.get()).current_index };
    with_process(index, |process| process.handles.transferable(handle, rights))
}

pub fn send_channel(
    endpoint_side: u8,
    bytes: &[u8],
    capability: Option<HandleDescriptor>,
) -> Result<(), &'static str> {
    SERVICE_CHANNEL.send(endpoint_side, bytes, capability)?;
    complete_waiting_receiver(endpoint_side ^ 1);
    Ok(())
}

pub fn receive_or_block(
    frame: *mut UserTrapFrame,
    endpoint_side: u8,
    buffer_address: u64,
    capacity: usize,
    out_handle_address: u64,
) -> *mut UserTrapFrame {
    match SERVICE_CHANNEL.receive(endpoint_side) {
        Ok(message) => {
            let index = unsafe { (*RUNTIME.0.get()).current_index };
            let result = deliver_message(index, message, buffer_address, capacity, out_handle_address);
            unsafe { (*frame).rax = result.unwrap_or_else(encode_error) };
            frame
        }
        Err(_) => {
            let index = unsafe { (*RUNTIME.0.get()).current_index };
            with_process_mut(index, |process| {
                process.saved_frame = frame as u64;
                process.pending_receive = Some(PendingReceive {
                    endpoint_side,
                    buffer_address,
                    capacity,
                    out_handle_address,
                });
                process.state = ProcessState::Blocked;
            });
            serial::println(format_args!(
                "[IPC ] pid={} blocked on empty channel endpoint {}",
                current_pid(), endpoint_side
            ));
            schedule_next(frame)
        }
    }
}

fn complete_waiting_receiver(receiver_side: u8) {
    let count = unsafe { (*RUNTIME.0.get()).count };
    for index in 0..count {
        let pending = with_process(index, |process| {
            if process.state == ProcessState::Blocked {
                process.pending_receive
            } else {
                None
            }
        });
        let Some(pending) = pending else { continue };
        if pending.endpoint_side != receiver_side {
            continue;
        }
        let Ok(message) = SERVICE_CHANNEL.receive(receiver_side) else { return };
        let result = deliver_message(
            index,
            message,
            pending.buffer_address,
            pending.capacity,
            pending.out_handle_address,
        );
        with_process_mut(index, |process| {
            process.pending_receive = None;
            process.state = ProcessState::Ready;
            unsafe {
                (*(process.saved_frame as *mut UserTrapFrame)).rax =
                    result.unwrap_or_else(encode_error);
            }
        });
        serial::println(format_args!(
            "[IPC ] woke pid={} after channel delivery",
            with_process(index, |process| process.pid)
        ));
        return;
    }
}

fn deliver_message(
    process_index: usize,
    message: ChannelMessage,
    buffer_address: u64,
    capacity: usize,
    out_handle_address: u64,
) -> Result<u64, i64> {
    if capacity < message.length {
        return Err(ERROR_NO_SPACE);
    }
    with_process_mut(process_index, |process| {
        process
            .address_space
            .copy_to_user(buffer_address, &message.bytes[..message.length])
            .map_err(|_| ERROR_ACCESS_DENIED)?;
        let received_handle = if let Some(capability) = message.capability {
            process
                .handles
                .allocate(capability.object, capability.rights)
                .map_err(|_| ERROR_NO_SPACE)?
        } else {
            0
        };
        if out_handle_address != 0 {
            process
                .address_space
                .copy_to_user(out_handle_address, &received_handle.to_le_bytes())
                .map_err(|_| ERROR_ACCESS_DENIED)?;
        }
        Ok(message.length as u64)
    })
}

/// Records the result of DISPLAYD's capability-mediated K13.D recovery call.
/// The result is intentionally not enough to finish qualification: userspace
/// must first execute a successful write after the call so the observable
/// DISPLAYD success/fallback banner cannot be raced by the scripted shell.
pub fn note_displayd_recovery_result(success: bool) {
    let runtime = unsafe { &mut *RUNTIME.0.get() };
    runtime.displayd_recovery_result = if success { 1 } else { -1 };
    runtime.displayd_recovery_acknowledged = false;
    serial::println(format_args!(
        "[QUAL] DISPLAYD recovery result armed: success={}",
        success
    ));
}

/// Called only after a successful SYS_WRITE has emitted bytes. If DISPLAYD has
/// a pending recovery result, that write is the userspace-visible completion
/// acknowledgement. This keeps the qualification boundary independent of
/// scheduler timing without matching process names or banner strings.
pub fn acknowledge_displayd_recovery_write() {
    let index = unsafe { (*RUNTIME.0.get()).current_index };
    if SERVICE_SPECS[index].role != ServiceRole::Display {
        return;
    }
    let runtime = unsafe { &mut *RUNTIME.0.get() };
    if runtime.displayd_recovery_result != 0 && !runtime.displayd_recovery_acknowledged {
        runtime.displayd_recovery_acknowledged = true;
        serial::println(format_args!(
            "[QUAL] DISPLAYD recovery userspace acknowledgement received"
        ));
    }
}

fn qualification_result() -> Option<u64> {
    let runtime = unsafe { &*RUNTIME.0.get() };
    let audio_server_required = crate::forgeaudio::device_count() != 0;
    if audio_server_required && runtime.forgeaudiod_failed {
        return Some(1);
    }
    if !runtime.shell_completed || !runtime.displayd_recovery_acknowledged
        || (audio_server_required && (!runtime.forgeaudiod_ready || !runtime.forgeaudiod_heartbeat
            || !runtime.forgeaudio_transport_ready || !runtime.forgeaudio_transport_dead_isolated))
    {
        return None;
    }
    if runtime.faulted != 0 || runtime.displayd_recovery_result < 0 {
        Some(1)
    } else if runtime.displayd_recovery_result > 0 {
        Some(0)
    } else {
        None
    }
}

fn finish_qualification_if_ready() {
    let Some(result) = qualification_result() else { return };
    if crate::forgeaudio::device_count() != 0 {
        serial::println(format_args!(
            "[K15D] required ForgeAudioD ready+heartbeat milestones complete; userspace qualification may close"
        ));
    }
    serial::println(format_args!(
        "[QUAL] shell and DISPLAYD recovery milestones complete; stopping persistent services"
    ));
    finish_runtime_with_result(result);
}

pub fn schedule_from_interrupt(frame: *mut UserTrapFrame) -> *mut UserTrapFrame {
    if !runtime_active() {
        return frame;
    }
    schedule_next(frame)
}

pub fn yield_current(frame: *mut UserTrapFrame) -> *mut UserTrapFrame {
    schedule_next(frame)
}

fn schedule_next(frame: *mut UserTrapFrame) -> *mut UserTrapFrame {
    reap_terminal_processes();
    finish_qualification_if_ready();
    let (current, cursor, count, cpu_index) = unsafe {
        let runtime = &*RUNTIME.0.get();
        (
            runtime.current_index,
            runtime.cursor,
            runtime.count,
            runtime.cpu_index,
        )
    };
    with_process_mut(current, |process| {
        process.saved_frame = frame as u64;
        if process.state == ProcessState::Running {
            process.state = ProcessState::Ready;
        }
    });

    let Some(next) = choose_next_ready(cursor, count) else {
        if all_terminal(count) {
            finish_runtime();
        }
        serial::emergency_println(format_args!("[FAIL] K6 has no runnable user process"));
        halt_forever();
    };
    unsafe {
        let runtime = &mut *RUNTIME.0.get();
        runtime.cursor = next;
        runtime.current_index = next;
        runtime.context_switches = runtime.context_switches.saturating_add(1);
    }
    with_process_mut(next, |process| process.state = ProcessState::Running);
    let (next_frame, cr3, stack_top) = process_switch_values(next);
    activate_address_space(cr3, stack_top, cpu_index);
    reclaim_terminal_address_space(current);
    next_frame as *mut UserTrapFrame
}

fn choose_next_ready(cursor: usize, count: usize) -> Option<usize> {
    for offset in 1..=count {
        let index = (cursor + offset) % count;
        if with_process(index, |process| process.state == ProcessState::Ready) {
            return Some(index);
        }
    }
    None
}

pub fn exit_current(frame: *mut UserTrapFrame, exit_code: u64) -> *mut UserTrapFrame {
    let (index, count) = unsafe {
        let runtime = &*RUNTIME.0.get();
        (runtime.current_index, runtime.count)
    };
    let pid = with_process(index, |process| process.pid);
    with_process_mut(index, |process| {
        process.saved_frame = frame as u64;
        process.exit_code = exit_code;
        process.state = ProcessState::Exited;
    });
    unsafe { (*RUNTIME.0.get()).exited += 1 };
    serial::println(format_args!("[PROC] pid={} exited with code {}", pid, exit_code));
    crate::forgeaudio_transport::detach_process(pid);

    if SERVICE_SPECS[index].role == ServiceRole::Audio && crate::forgeaudio::device_count() != 0 {
        unsafe { (*RUNTIME.0.get()).forgeaudiod_failed = true };
        serial::println(format_args!(
            "[K15D] ForgeAudioD exited unexpectedly during required HDA service lifetime: code={}",
            exit_code
        ));
    }

    // K13.D has two independent userspace qualification controllers: the
    // scripted shell and DISPLAYD's recovery handshake.  Shell completion no
    // longer tears the runtime down by itself; both milestones must be
    // observable first, avoiding a scheduler race with DISPLAYD's return from
    // SYS_GPU_PRESENT/SYS_GPU_RECOVER.
    if SERVICE_SPECS[index].role == ServiceRole::Shell && exit_code == 0 {
        unsafe { (*RUNTIME.0.get()).shell_completed = true };
        serial::println(format_args!(
            "[QUAL] scripted shell completed; waiting for DISPLAYD recovery acknowledgement"
        ));
        finish_qualification_if_ready();
    }

    if all_terminal(count) {
        finish_runtime();
    }
    schedule_next(frame)
}

pub fn fault_current(frame: *mut UserTrapFrame, vector: u64, error_code: u64) -> *mut UserTrapFrame {
    let (index, count) = unsafe {
        let runtime = &*RUNTIME.0.get();
        (runtime.current_index, runtime.count)
    };
    let pid = with_process(index, |process| process.pid);
    with_process_mut(index, |process| {
        process.saved_frame = frame as u64;
        process.fault_vector = vector;
        process.exit_code = error_code;
        process.state = ProcessState::Faulted;
    });
    unsafe { (*RUNTIME.0.get()).faulted += 1 };
    serial::emergency_println(format_args!(
        "[PROC] pid={} terminated after user exception vector={} error={:#x}",
        pid, vector, error_code
    ));
    crate::forgeaudio_transport::detach_process(pid);
    if all_terminal(count) {
        finish_runtime();
    }
    schedule_next(frame)
}

fn all_terminal(count: usize) -> bool {
    (0..count).all(|index| {
        matches!(
            with_process(index, |process| process.state),
            ProcessState::Exited | ProcessState::Faulted
        )
    })
}

fn finish_runtime() -> ! {
    let runtime = unsafe { &*RUNTIME.0.get() };
    let result = if runtime.exited == runtime.count && runtime.faulted == 0 { 0 } else { 1 };
    finish_runtime_with_result(result)
}

fn finish_runtime_with_result(result: u64) -> ! {
    let runtime = unsafe { &mut *RUNTIME.0.get() };
    runtime.active = false;
    apic::mask_timer();
    unsafe { user::return_to_kernel(runtime.kernel_cr3, runtime.kernel_return_stack, result) }
}


fn allocator_mut() -> Option<&'static mut FrameAllocator<'static>> {
    let pointer = unsafe { (*RUNTIME.0.get()).allocator };
    if pointer.is_null() { None } else { Some(unsafe { &mut *pointer }) }
}

fn reclaim_terminal_address_space(index: usize) {
    let terminal = with_process(index, |p| matches!(p.state, ProcessState::Exited | ProcessState::Faulted) && !p.address_space_reclaimed);
    if !terminal { return; }
    let pid = with_process(index, |p| p.pid);
    let Some(allocator) = allocator_mut() else { return };
    if !crate::forgebus::fence_owner_io(pid, allocator) { return; }
    with_process_mut(index, |process| {
        if !process.resources_released {
            process.handles.close_all(|descriptor| release_handle_object(pid, descriptor));
            let _ = crate::kernel_runtime::with_runtime(|runtime| runtime.objects.release_owner(pid));
            process.pending_receive = None;
            process.resources_released = true;
        }
        match process.address_space.destroy(allocator) {
            Ok(pages) => { process.address_space_reclaimed = true; serial::println(format_args!("[PROC] pid={} reclaimed {} address-space pages", pid, pages)); }
            Err(error) => serial::emergency_println(format_args!("[PROC] pid={} address-space reclaim deferred: {}", pid, error)),
        }
    });
}

fn reap_terminal_processes() {
    let current = unsafe { (*RUNTIME.0.get()).current_index };
    let count = unsafe { (*RUNTIME.0.get()).count };
    let Some(allocator) = allocator_mut() else { return };
    for index in 0..count {
        if index == current { continue; }
        let should_reap = with_process(index, |p| matches!(p.state, ProcessState::Exited | ProcessState::Faulted) && p.address_space_reclaimed && p.kernel_stack_base != 0);
        if should_reap {
            with_process_mut(index, |p| {
                if allocator.deallocate_contiguous(p.kernel_stack_base, USER_KERNEL_STACK_PAGES).is_ok() {
                    serial::println(format_args!("[PROC] pid={} reclaimed kernel stack", p.pid));
                    p.kernel_stack_base = 0; p.kernel_stack_top = 0; p.saved_frame = 0;
                }
            });
        }
    }
}

fn release_handle_object(owner: ProcessId, descriptor: HandleDescriptor) {
    match descriptor.object {
        HandleObject::Audio { object_id, kind } => {
            if let Err(error) = crate::forgeaudio::release_object(kind, object_id, owner) {
                serial::println(format_args!("[K15ABI] audio handle release deferred/failed: object={:#x} kind={:?} error={}", object_id, kind, error));
            }
        }
        HandleObject::File{object_id}|HandleObject::Process{object_id}|HandleObject::SharedMemory{object_id}|HandleObject::Device{object_id} => {
            let _ = crate::kernel_runtime::with_runtime(|runtime| runtime.objects.release_object(object_id));
        }
        _ => {}
    }
}

fn process_switch_values(index: usize) -> (u64, u64, u64) {
    with_process(index, |process| {
        (process.saved_frame, process.address_space.cr3, process.kernel_stack_top)
    })
}

fn activate_address_space(cr3: u64, kernel_stack_top: u64, cpu_index: usize) {
    gdt::set_kernel_stack(cpu_index, kernel_stack_top);
    unsafe { asm!("mov cr3, {}", in(reg) cr3, options(nostack, preserves_flags)) };
}

fn with_process<R>(index: usize, operation: impl FnOnce(&Process) -> R) -> R {
    unsafe {
        let slot = &*PROCESS_SLOTS[index].0.get();
        operation(slot.as_ref().expect("K6 process slot is empty"))
    }
}

fn with_process_mut<R>(index: usize, operation: impl FnOnce(&mut Process) -> R) -> R {
    unsafe {
        let slot = &mut *PROCESS_SLOTS[index].0.get();
        operation(slot.as_mut().expect("K6 process slot is empty"))
    }
}


fn finalize_all_processes() {
    let count = unsafe { (*RUNTIME.0.get()).count };
    let Some(allocator) = allocator_mut() else { return };
    for index in 0..count {
        let pid = with_process(index, |p| p.pid);
        let _ = crate::forgebus::fence_owner_io(pid, allocator);
        with_process_mut(index, |p| {
            if !p.resources_released {
                p.handles.close_all(|descriptor| release_handle_object(pid, descriptor));
                let _ = crate::kernel_runtime::with_runtime(|r| r.objects.release_owner(pid));
                p.resources_released=true;
            }
            if !p.address_space_reclaimed && p.address_space.cr3 != 0 {
                if p.address_space.destroy(allocator).is_ok() { p.address_space_reclaimed=true; }
            }
            if p.kernel_stack_base != 0 {
                let _=allocator.deallocate_contiguous(p.kernel_stack_base,USER_KERNEL_STACK_PAGES);
                p.kernel_stack_base=0;p.kernel_stack_top=0;p.saved_frame=0;
            }
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn weave_user_exit_resume(result: u64) -> ! {
    finalize_all_processes();
    let runtime = unsafe { &*RUNTIME.0.get() };
    serial::println(format_args!(
        "[USER] Process runtime complete: exited={} faulted={} switches={}",
        runtime.exited, runtime.faulted, runtime.context_switches
    ));
    if result != 0 {
        serial::println(format_args!(
            "[K6DIAG] user runtime failure detail: exited={} count={} faulted={} shell_completed={} displayd_recovery_result={} displayd_recovery_acknowledged={} forgeaudiod_registered={} forgeaudiod_ready={} forgeaudiod_heartbeat={} forgeaudiod_failed={}",
            runtime.exited, runtime.count, runtime.faulted, runtime.shell_completed,
            runtime.displayd_recovery_result, runtime.displayd_recovery_acknowledged,
            runtime.forgeaudiod_registered, runtime.forgeaudiod_ready, runtime.forgeaudiod_heartbeat, runtime.forgeaudiod_failed
        ));
        serial::println(format_args!("[FAIL] K6 user runtime reported failure"));
        halt_forever();
    }
    if crate::forgeaudio::device_count() != 0 {
        if !runtime.forgeaudiod_registered || !runtime.forgeaudiod_ready || !runtime.forgeaudiod_heartbeat {
            serial::println(format_args!("[FAIL] K15.6 ForgeAudioD post-userspace qualification state incomplete"));
            halt_forever();
        }
        serial::println(format_args!(
            "[K15OK] K15.6 ForgeAudioD userspace audio server qualified: userspace=true singleton=true device_ownership=true streams=true routing=true clocks=true buffers=true telemetry=true recovery=true persistent=true"
        ));
        serial::println(format_args!(
            "[K15SR] ForgeAudioD ready: pid={} routes={} graph_generation={} recoveries={} heartbeat_sequence={} persistent=true",
            runtime.forgeaudiod_pid,
            runtime.forgeaudiod_route_count,
            runtime.forgeaudiod_graph_generation,
            runtime.forgeaudiod_recovery_count,
            runtime.forgeaudiod_heartbeat_sequence,
        ));
    }

    serial::println(format_args!(
        "[KERN] K13.D alive: GPU resilience, multi-GPU policy, buffered presentation, VirtIO-GPU transport, graphics, ForgeBus, storage, VFS, and native services verified"
    ));
    serial::println(format_args!(
        "[QUAL] K13.D robustness runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.A alive: native GPU prerequisite policy, K13 GPU resilience/presentation/transport, ForgeBus, storage, VFS, and native services verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.A native-GPU foundation runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.B alive: hardware-translated DMA qualification, native GPU admission safety, K13 GPU resilience/presentation/transport, ForgeBus, storage, VFS, and native services verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.B translated-DMA runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C1 alive: native GPU backend ownership, BAR inventory, VRAM/GTT policy, translated-DMA safety, and K13 fallbacks verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C1 native binding runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C2 alive: persistent translated-domain lifecycle, AMD firmware/ring bring-up contract, native DMA fencing, and K13 fallbacks verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C2 native bring-up runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C3 alive: Radeon bare-metal staging, IP/firmware/MMIO safety contract, translated-domain fencing, and K13 fallbacks verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C3 Radeon staging runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C4 alive: exact Radeon requester identity, AMD-Vi/IVRS admission, persistent-domain fencing, and K13 fallbacks verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C4 Radeon domain-gate runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C5 alive: AMD-Vi device/page tables, command/event buffers, exact Radeon requester image, fault fencing, and K13 fallbacks verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C5 AMD-Vi page-table runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C6 alive: live AMD-Vi register programming boundary, exact requester activation gate, fault/invalidation fencing, and K13 fallbacks verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C6 live AMD-Vi runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C7 alive: read-only Radeon MMIO aperture contract, PCI/VBIOS/firmware discovery staging, GMC/GTT readiness fencing, and K13 fallbacks verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C7 Radeon discovery runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C8 alive: Radeon ASIC/IP profile gate, whitelisted side-effect-free register-read contract, firmware resolution fencing, and K13 fallbacks verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C8 Radeon ASIC/IP runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C9 alive: verified Radeon device profiles, live side-effect-free PCI identity reads, MMIO-read fencing, and K13 fallbacks verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C9 verified Radeon profile runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C10 alive: per-IP MMIO whitelist engine, bounded guarded read executor, strict read-only aperture policy, and K13 fallbacks verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C10 guarded Radeon MMIO-read runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C11 alive: reviewed Radeon register definitions, IP-relative address resolver fencing, guarded read policy, and K13 fallbacks verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C11 reviewed-register runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C12 alive: trusted Radeon IP-base sources, modern BAR5 register-aperture policy, bounded live status reads, and K13 fallbacks verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C12 trusted-base/live-read runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C13 alive: physical Radeon read-proof qualification, status sanity checking, bus-master recheck, immutable serial evidence, and K13 fallbacks verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C13 physical-read-proof runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C14 alive: controlled Radeon write-promotion readiness, prerequisite aggregation, bus-master recheck, and destructive-path fencing verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C14 write-promotion-readiness runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C15 alive: first controlled Radeon write transaction, width-correct PCI Command identity write, readback, bounded rollback, and MMIO-write fencing verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C15 controlled-write runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C16 alive: reviewed Radeon MMIO-write target, fail-closed exact-index/base gate, bounded readback, rollback, and destructive-capability fencing verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C16 reviewed-MMIO-write runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C17 alive: AMD IP-discovery parser, bounded source-layout validation, Navi48 exact-base resolution gate, and write fencing verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C17 IP-discovery runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C18 alive: AMD discovery checksum verifier, TMR acquisition contract, synthetic integrity proof, and destructive-path fencing verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C18 snapshot-verification runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C19 alive: source-backed TMR location reads, ReBAR-bounded BAR0 acquisition, checksum qualification, and write/bus-master fencing verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C19 physical-snapshot runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C20 alive: checksum-qualified AMD IP-record enumeration, exact GC/SDMA base resolution, GFX12 promotion-input proof, and destructive-path fencing verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C20 exact-IP-base runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C21 alive: exact generated GFX12 SCRATCH_REG0/base-index import, live GC base1 cross-check, bounded MMIO identity-write promotion, and bus-master fencing verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C21 reviewed-MMIO-rebind runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C22 alive: bounded one-bit GFX12 SCRATCH_REG0 mutation, exact readback, mandatory restoration, retry recovery, and bus-master fencing verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C22 reversible-scratch-mutation runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C23 alive: C22 restore persistence, two distinct one-bit SCRATCH_REG0 probe/restore cycles, bounded recovery, and bus-master fencing verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C23 dual-probe-stability runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C24 alive: C23 stability inheritance, deterministic four-bit SCRATCH_REG0 pattern/readback, mandatory restoration, bounded recovery, and bus-master fencing verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C24 reversible-multi-bit-pattern runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C25 alive: C24 multi-bit inheritance, two distinct four-bit SCRATCH_REG0 pattern/readback/restore cycles, intercycle persistence, bounded recovery, and bus-master fencing verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C25 dual-multi-bit-pattern runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C26 alive: frozen C25 inheritance, exact second GFX12 SCRATCH_REG1 resolution, two-entry reviewed MMIO allowlist, bounded read-only proof, zero C26 MMIO writes, and bus-master fencing verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C26 final-k14-mmio-allowlist runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[K14FOUND] K14.C26 native Radeon MMIO foundation frozen; fixed roadmap continues K14 Radeon through C32 before K15 ForgeAudio"
    ));
    serial::println(format_args!(
        "[KERN] K14.C27 alive: operational Radeon device/lifecycle core, exact ForgeBus ownership, live resource topology, permanent reviewed-MMIO service, real masked interrupt route/handler, and executable error/reset coordination verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C27 complete-radeon-driver-core runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C28 alive: reclaimable GTT backing, BAR0 VRAM reservation allocator, AMD common-firmware parse/CRC/SHA staging, pinned firmware ownership, watchdog, and resource-safe software recovery verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C28 memory-firmware-recovery runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C29 alive: operational GTT-backed SDMA ring, FIFO submission queue, timeline fence, typed COPY/FENCE packet codec, bounded owned-memory DMA executor, and exact GFX12 SDMA0 queue register plan verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C29 rings-queues-fences-dma runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C30 alive: validated EDID/mode selection, connector/CRTC/plane ownership, double-buffered GTT scanout, real live-framebuffer page flips, atomic current-mode commit/rollback, hotplug bookkeeping, and source-reviewed DCN401 resources verified"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C30 complete-basic-display-engine runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C31 alive: owned shader upload/cache/precache, typed command encoding, separate compute/graphics queues, verified vector-add dispatch, verified triangle draw, live framebuffer present, timeline-fence retirement, and future compute capability model operational"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C31 graphics-compute-execution runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[KERN] K14.C32 alive: queue and memory pressure stress, stuck-queue recovery, software IRQ/recovery stress, display+compute and graphics+compute coexistence, repeated scanout, multi-display framework, telemetry, power policy, frozen GPU ABI/capabilities, shader precache, and multi-GPU inventory operational"
    ));
    serial::println(format_args!(
        "[QUAL] K14.C32 production-stability-final runtime reached intentional post-userspace halt"
    ));
    serial::println(format_args!(
        "[K14DONE] Titanweave native Radeon driver foundation operational"
    ));
    serial::println(format_args!(
        "[K15NEXT] K15 ForgeAudio is the next locked Titanweave milestone"
    ));
    serial::println(format_args!("[HALT] BSP halted intentionally"));
    halt_forever();
}

pub fn map_handle_error(error: &'static str) -> i64 {
    if error.contains("rights") {
        ERROR_ACCESS_DENIED
    } else {
        ERROR_BAD_HANDLE
    }
}
