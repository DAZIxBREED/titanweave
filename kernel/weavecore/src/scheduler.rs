use crate::acpi::MAX_CPUS;
use crate::arch::x86_64::apic;
use crate::arch::x86_64::gdt::{KERNEL_CODE_SELECTOR, KERNEL_DATA_SELECTOR};
use crate::arch::x86_64::idt::{InterruptFrame, RESCHEDULE_VECTOR, TIMER_VECTOR};
use crate::arch::x86_64::{self, halt_forever, halt_once, pause};
use crate::memory::FrameAllocator;
use crate::objects::{ObjectId, DEMO_EVENT};
use crate::percpu;
use crate::serial;
use core::arch::asm;
use core::cell::UnsafeCell;
use core::mem::size_of;
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

pub type TaskId = u32;

const MAX_TASKS: usize = 16;
const TASK_STACK_PAGES: u64 = 16;
const TASK_STACK_SIZE: u64 = TASK_STACK_PAGES * 4096;
const IDLE_TASK_ID: TaskId = 0;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum TaskState {
    Empty,
    Ready,
    Running,
    Blocked,
    Finished,
}

#[derive(Clone, Copy)]
struct Task {
    id: TaskId,
    state: TaskState,
    saved_frame: u64,
    stack_base: u64,
    affinity_cpu: usize,
    wait_object: ObjectId,
    runtime_ticks: u64,
    started: bool,
}

impl Task {
    const EMPTY: Self = Self {
        id: 0,
        state: TaskState::Empty,
        saved_frame: 0,
        stack_base: 0,
        affinity_cpu: 0,
        wait_object: 0,
        runtime_ticks: 0,
        started: false,
    };
}

struct TaskTable {
    tasks: [Task; MAX_TASKS],
    count: usize,
    next_id: TaskId,
}

impl TaskTable {
    const fn new() -> Self {
        Self {
            tasks: [Task::EMPTY; MAX_TASKS],
            count: 0,
            next_id: 1,
        }
    }
}

#[derive(Clone, Copy)]
struct CpuRunQueue {
    entries: [usize; MAX_TASKS],
    len: usize,
    cursor: usize,
    current_task_index: usize,
    timer_ticks: u64,
    preemptions: u64,
    initialized: bool,
}

impl CpuRunQueue {
    const fn new() -> Self {
        Self {
            entries: [0; MAX_TASKS],
            len: 0,
            cursor: 0,
            current_task_index: 0,
            timer_ticks: 0,
            preemptions: 0,
            initialized: false,
        }
    }

    fn push(&mut self, task_index: usize) -> Result<(), &'static str> {
        if self.len == MAX_TASKS {
            return Err("per-CPU run queue is full");
        }
        self.entries[self.len] = task_index;
        self.len += 1;
        Ok(())
    }
}

struct TaskTableCell(UnsafeCell<TaskTable>);
unsafe impl Sync for TaskTableCell {}

struct RunQueueCell(UnsafeCell<CpuRunQueue>);
unsafe impl Sync for RunQueueCell {}

static TASK_TABLE: TaskTableCell = TaskTableCell(UnsafeCell::new(TaskTable::new()));
static RUN_QUEUES: [RunQueueCell; MAX_CPUS] =
    [const { RunQueueCell(UnsafeCell::new(CpuRunQueue::new())) }; MAX_CPUS];
static DEMO_COMPLETE: AtomicBool = AtomicBool::new(false);
static FIRST_CONTEXT_SWITCH_LOGGED: AtomicBool = AtomicBool::new(false);

pub struct SchedulerReport {
    pub ticks: u64,
    pub preemptions: u64,
    pub tasks_completed: usize,
}

pub fn run_scheduler_self_test(
    allocator: &mut FrameAllocator<'_>,
    cpu_index: usize,
    timer_initial_count: u32,
) -> Result<SchedulerReport, &'static str> {
    assert!(
        !x86_64::interrupts_enabled(),
        "scheduler initialization requires interrupts disabled"
    );
    prepare_cpu_queue(cpu_index)?;
    create_task(allocator, cpu_index, demo_task_a)?;
    create_task(allocator, cpu_index, demo_task_b)?;

    let event = DEMO_EVENT.header();
    serial::println(format_args!(
        "[OBJ ] Event id={} kind={:?} refs={}",
        event.id(),
        event.kind(),
        event.reference_count()
    ));
    serial::println(format_args!(
        "[SCHED] Starting 100 Hz scheduler self-test on logical CPU {}",
        cpu_index
    ));

    DEMO_COMPLETE.store(false, Ordering::Release);
    apic::start_periodic_timer(TIMER_VECTOR, timer_initial_count);
    x86_64::enable_interrupts();

    while !DEMO_COMPLETE.load(Ordering::Acquire) {
        halt_once();
    }

    x86_64::disable_interrupts();
    apic::mask_timer();

    let (ticks, preemptions) = unsafe {
        let queue = &*RUN_QUEUES[cpu_index].0.get();
        (queue.timer_ticks, queue.preemptions)
    };
    let tasks_completed = unsafe {
        let table = &*TASK_TABLE.0.get();
        table.tasks[1..table.count]
            .iter()
            .filter(|task| task.state == TaskState::Finished)
            .count()
    };

    serial::println(format_args!(
        "[SCHED] Scheduler self-test complete: ticks={} preemptions={} workers={}",
        ticks, preemptions, tasks_completed
    ));

    Ok(SchedulerReport {
        ticks,
        preemptions,
        tasks_completed,
    })
}

fn prepare_cpu_queue(cpu_index: usize) -> Result<(), &'static str> {
    if cpu_index >= MAX_CPUS {
        return Err("scheduler CPU index outside static table");
    }

    unsafe {
        *TASK_TABLE.0.get() = TaskTable::new();
        *RUN_QUEUES[cpu_index].0.get() = CpuRunQueue::new();

        let table = &mut *TASK_TABLE.0.get();
        table.tasks[0] = Task {
            id: IDLE_TASK_ID,
            state: TaskState::Running,
            saved_frame: 0,
            stack_base: 0,
            affinity_cpu: cpu_index,
            wait_object: 0,
            runtime_ticks: 0,
            started: true,
        };
        table.count = 1;

        let queue = &mut *RUN_QUEUES[cpu_index].0.get();
        queue.push(0)?;
        queue.current_task_index = 0;
        queue.cursor = 0;
        queue.initialized = true;
    }

    percpu::set_current_task(cpu_index, IDLE_TASK_ID);
    Ok(())
}

fn create_task(
    allocator: &mut FrameAllocator<'_>,
    cpu_index: usize,
    entry: extern "C" fn() -> !,
) -> Result<TaskId, &'static str> {
    let stack_base = allocator
        .allocate_contiguous(TASK_STACK_PAGES)
        .ok_or("no contiguous pages available for task stack")?;
    unsafe { ptr::write_bytes(stack_base as *mut u8, 0, TASK_STACK_SIZE as usize) };
    let stack_top = stack_base + TASK_STACK_SIZE;
    let saved_frame = build_initial_frame(stack_top, entry as usize as u64)?;

    unsafe {
        let table = &mut *TASK_TABLE.0.get();
        if table.count >= MAX_TASKS {
            return Err("kernel task table is full");
        }

        let task_id = table.next_id;
        table.next_id = table.next_id.checked_add(1).ok_or("task ID overflow")?;
        let task_index = table.count;
        table.tasks[task_index] = Task {
            id: task_id,
            state: TaskState::Ready,
            saved_frame,
            stack_base,
            affinity_cpu: cpu_index,
            wait_object: 0,
            runtime_ticks: 0,
            started: false,
        };
        table.count += 1;

        let queue = &mut *RUN_QUEUES[cpu_index].0.get();
        queue.push(task_index)?;
        Ok(task_id)
    }
}

fn build_initial_frame(stack_top: u64, entry: u64) -> Result<u64, &'static str> {
    if size_of::<InterruptFrame>() != 22 * size_of::<u64>() {
        return Err("interrupt-frame layout changed unexpectedly");
    }

    // IRETQ in 64-bit mode restores SS:RSP from the interrupt frame even for
    // a CPL0 -> CPL0 return. Seed an explicit task RSP rather than relying on
    // the address immediately following RIP/CS/RFLAGS. Keep it 8 mod 16 to
    // satisfy the SysV x86-64 function-entry convention.
    let initial_rsp = (stack_top & !0x0f).checked_sub(8).ok_or("task stack underflow")?;
    let frame_end = initial_rsp;
    let frame_address = frame_end
        .checked_sub(size_of::<InterruptFrame>() as u64)
        .ok_or("task interrupt frame underflow")?;

    unsafe {
        ptr::write(
            frame_address as *mut InterruptFrame,
            InterruptFrame::for_kernel_task(entry, initial_rsp),
        );
    }
    Ok(frame_address)
}

#[unsafe(no_mangle)]
pub extern "C" fn weave_schedule_dispatch(frame: *mut InterruptFrame) -> *mut InterruptFrame {
    if unsafe { (*frame).cs & 3 == 3 } && crate::process::runtime_active() {
        if unsafe { (*frame).vector == TIMER_VECTOR as u64 } {
            apic::end_of_interrupt();
        }
        return crate::process::schedule_from_interrupt(frame.cast::<crate::user::UserTrapFrame>())
            .cast::<InterruptFrame>();
    }
    let cpu_index = percpu::bsp_index();

    unsafe {
        let table = &mut *TASK_TABLE.0.get();
        let queue = &mut *RUN_QUEUES[cpu_index].0.get();
        if !queue.initialized || queue.len == 0 {
            if (*frame).vector == TIMER_VECTOR as u64 {
                apic::end_of_interrupt();
            }
            return frame;
        }

        let current_index = queue.current_task_index;
        let timer_interrupt = (*frame).vector == TIMER_VECTOR as u64;
        {
            let current = &mut table.tasks[current_index];
            current.saved_frame = frame as u64;

            if timer_interrupt {
                queue.timer_ticks = queue.timer_ticks.saturating_add(1);
                current.runtime_ticks = current.runtime_ticks.saturating_add(1);
                percpu::increment_tick(cpu_index);
                apic::end_of_interrupt();
            }

            if current.state == TaskState::Running {
                current.state = TaskState::Ready;
            }
        }

        let (next_position, next_index) = choose_next(table, queue)
            .unwrap_or((queue.cursor, current_index));
        if !matches!(
            table.tasks[next_index].state,
            TaskState::Ready | TaskState::Running
        ) {
            // The BSP idle task is permanently runnable and should make this
            // impossible. Returning the current frame is safer than jumping to
            // an invalid stack if a future change violates that invariant.
            table.tasks[current_index].state = TaskState::Running;
            return frame;
        }

        if timer_interrupt && next_index != current_index {
            queue.preemptions = queue.preemptions.saturating_add(1);
        }

        let next = &mut table.tasks[next_index];
        if next.id != IDLE_TASK_ID {
            if let Err(error) = validate_kernel_task_frame(next) {
                serial::emergency_println(format_args!(
                    "[SCHED] Refusing invalid task frame: task={} frame={:#018x} stack={:#018x} error={}",
                    next.id, next.saved_frame, next.stack_base, error
                ));
                halt_forever();
            }
            if !FIRST_CONTEXT_SWITCH_LOGGED.swap(true, Ordering::AcqRel) {
                let initial = &*(next.saved_frame as *const InterruptFrame);
                serial::emergency_println(format_args!(
                    "[SCHED] First switch: task={} frame={:#018x} stack={:#018x} rip={:#018x} cs={:#x} rflags={:#x} rsp={:#018x} ss={:#x}",
                    next.id,
                    next.saved_frame,
                    next.stack_base,
                    initial.rip,
                    initial.cs,
                    initial.rflags,
                    initial.rsp,
                    initial.ss
                ));
            }
        }
        next.state = TaskState::Running;
        if next.id != IDLE_TASK_ID {
            next.started = true;
        }
        queue.cursor = next_position;
        queue.current_task_index = next_index;
        percpu::set_current_task(cpu_index, next.id);
        next.saved_frame as *mut InterruptFrame
    }
}

fn validate_kernel_task_frame(task: &Task) -> Result<(), &'static str> {
    if task.saved_frame == 0 || task.stack_base == 0 {
        return Err("task frame or stack base is zero");
    }
    let stack_end = task.stack_base.checked_add(TASK_STACK_SIZE).ok_or("task stack overflow")?;
    let frame_end = task.saved_frame
        .checked_add(size_of::<InterruptFrame>() as u64)
        .ok_or("task frame overflow")?;
    if task.saved_frame < task.stack_base || frame_end > stack_end {
        return Err("task interrupt frame is outside its stack");
    }
    if task.saved_frame & 7 != 0 {
        return Err("task interrupt frame is not naturally aligned");
    }

    let frame = unsafe { &*(task.saved_frame as *const InterruptFrame) };
    if frame.cs != KERNEL_CODE_SELECTOR as u64 {
        return Err("task frame has unexpected kernel CS");
    }
    if !is_canonical(frame.rip) {
        return Err("task frame RIP is non-canonical");
    }
    if frame.rflags & 0x2 == 0 {
        return Err("task frame RFLAGS fixed bit is clear");
    }
    if frame.ss != KERNEL_DATA_SELECTOR as u64 {
        return Err("task frame has unexpected kernel SS");
    }
    if !is_canonical(frame.rsp) {
        return Err("task frame RSP is non-canonical");
    }
    if frame.rsp < task.stack_base || frame.rsp > stack_end {
        return Err("task frame RSP is outside its stack");
    }
    // The SysV 8-mod-16 RSP rule applies at a freshly entered function. Once a
    // task has executed, an interrupt or software reschedule can capture RSP at
    // any compiler-selected point inside that function. Requiring entry
    // alignment on a resumed hardware frame incorrectly rejects valid tasks.
    if !task.started && frame.rsp & 0xf != 8 {
        return Err("fresh task frame RSP violates SysV entry alignment");
    }
    Ok(())
}

const fn is_canonical(address: u64) -> bool {
    let upper = address >> 48;
    upper == 0 || upper == 0xffff
}

fn choose_next(table: &TaskTable, queue: &CpuRunQueue) -> Option<(usize, usize)> {
    for offset in 1..=queue.len {
        let position = (queue.cursor + offset) % queue.len;
        let task_index = queue.entries[position];
        let task = table.tasks[task_index];
        if task.affinity_cpu == percpu::bsp_index()
            && matches!(task.state, TaskState::Ready | TaskState::Running)
        {
            return Some((position, task_index));
        }
    }
    None
}

#[must_use]
pub fn current_task_id() -> TaskId {
    let cpu_index = percpu::bsp_index();
    unsafe {
        let queue = &*RUN_QUEUES[cpu_index].0.get();
        let table = &*TASK_TABLE.0.get();
        table.tasks[queue.current_task_index].id
    }
}

/// Mark the current task blocked. The caller must already have local
/// interrupts disabled and must immediately enter the reschedule vector after
/// releasing any wait-queue lock.
pub fn prepare_block_current(wait_object: ObjectId) {
    assert!(
        !x86_64::interrupts_enabled(),
        "blocking transition requires interrupts disabled"
    );
    let cpu_index = percpu::bsp_index();
    unsafe {
        let queue = &*RUN_QUEUES[cpu_index].0.get();
        let table = &mut *TASK_TABLE.0.get();
        let task = &mut table.tasks[queue.current_task_index];
        assert!(task.id != IDLE_TASK_ID, "idle task cannot block");
        task.wait_object = wait_object;
        task.state = TaskState::Blocked;
    }
}

pub fn reschedule_blocked_current() {
    unsafe {
        asm!(
            "int {vector}",
            vector = const RESCHEDULE_VECTOR,
        );
    }
}

pub fn wake_task(task_id: TaskId) {
    assert!(
        !x86_64::interrupts_enabled(),
        "wake_task requires an interrupt-disabled scheduler critical section"
    );
    unsafe {
        let table = &mut *TASK_TABLE.0.get();
        for task in &mut table.tasks[..table.count] {
            if task.id == task_id && task.state == TaskState::Blocked {
                task.wait_object = 0;
                task.state = TaskState::Ready;
                return;
            }
        }
    }
}

fn finish_current() -> ! {
    let saved_flags = x86_64::save_and_disable_interrupts();
    let cpu_index = percpu::bsp_index();
    unsafe {
        let queue = &*RUN_QUEUES[cpu_index].0.get();
        let table = &mut *TASK_TABLE.0.get();
        let current = &mut table.tasks[queue.current_task_index];
        current.state = TaskState::Finished;
        current.wait_object = 0;

        let complete = table.tasks[1..table.count]
            .iter()
            .all(|task| task.state == TaskState::Finished);
        if complete {
            DEMO_COMPLETE.store(true, Ordering::Release);
        }
    }

    // This interrupt cannot return to a finished task. Keep saved_flags only to
    // make the intentional interrupt-state transition explicit.
    let _ = saved_flags;
    reschedule_blocked_current();
    halt_forever();
}

fn wait_ticks(delta: u64) {
    let cpu_index = percpu::bsp_index();
    let target = percpu::ticks(cpu_index).saturating_add(delta);
    while percpu::ticks(cpu_index) < target {
        pause();
    }
}

extern "C" fn demo_task_a() -> ! {
    serial::println(format_args!("[TASK] A waiting on kernel event 1"));
    if let Err(error) = DEMO_EVENT.wait() {
        serial::println(format_args!("[TASK] A event wait failed: {error}"));
        finish_current();
    }
    serial::println(format_args!("[TASK] A woke after event signal"));
    for phase in 0..3 {
        wait_ticks(3);
        serial::println(format_args!("[TASK] A preemptible phase {}", phase));
    }
    finish_current();
}

extern "C" fn demo_task_b() -> ! {
    serial::println(format_args!("[TASK] B running before signal"));
    wait_ticks(5);
    DEMO_EVENT.signal();
    serial::println(format_args!("[TASK] B signaled kernel event 1"));
    for phase in 0..3 {
        wait_ticks(2);
        serial::println(format_args!("[TASK] B preemptible phase {}", phase));
    }
    finish_current();
}
