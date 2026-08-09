use crate::acpi::MAX_CPUS;
use crate::arch::x86_64::apic;
use crate::arch::x86_64::gdt::{KERNEL_CODE_SELECTOR, KERNEL_DATA_SELECTOR};
use crate::arch::x86_64::idt::{InterruptFrame, RESCHEDULE_VECTOR, TIMER_VECTOR};
use crate::arch::x86_64::{self, halt_forever, halt_once, pause};
use crate::memory::FrameAllocator;
use crate::objects::{ObjectId, DEMO_EVENT};
use crate::percpu;
use crate::rt_mutex::RtMutex;
use crate::serial;
use core::arch::asm;
use core::cell::UnsafeCell;
use core::mem::size_of;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

pub type TaskId = u32;

const MAX_TASKS: usize = 16;
const TASK_STACK_PAGES: u64 = 16;
const TASK_STACK_SIZE: u64 = TASK_STACK_PAGES * 4096;
const IDLE_TASK_ID: TaskId = 0;
const RT_PERIOD_WAIT_OBJECT: ObjectId = 0x15_0000_0001;
const NO_AUDIO_CPU: usize = usize::MAX;

pub const FORGEAUDIO_RT_TICK_HZ: u32 = 1_000;
pub const FORGEAUDIO_RT_MAX_PRIORITY: u8 = 31;
pub const FORGEAUDIO_RT_MIN_PRIORITY: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SchedulingClass {
    Normal = 0,
    RealtimeAudio = 1,
}

#[derive(Clone, Copy, Debug)]
pub struct RtTaskConfig {
    pub priority: u8,
    pub period_ticks: u64,
    pub budget_ticks: u64,
    pub deadline_ticks: u64,
    pub release_offset_ticks: u64,
}

impl RtTaskConfig {
    pub const fn audio(
        priority: u8,
        period_ticks: u64,
        budget_ticks: u64,
        deadline_ticks: u64,
        release_offset_ticks: u64,
    ) -> Self {
        Self {
            priority,
            period_ticks,
            budget_ticks,
            deadline_ticks,
            release_offset_ticks,
        }
    }

    fn validate(self) -> Result<(), &'static str> {
        if self.priority < FORGEAUDIO_RT_MIN_PRIORITY || self.priority > FORGEAUDIO_RT_MAX_PRIORITY {
            return Err("ForgeAudio RT priority outside supported range");
        }
        if self.period_ticks == 0 {
            return Err("ForgeAudio RT period must be nonzero");
        }
        if self.budget_ticks == 0 || self.budget_ticks > self.period_ticks {
            return Err("ForgeAudio RT budget must be within its period");
        }
        if self.deadline_ticks == 0 || self.deadline_ticks > self.period_ticks {
            return Err("ForgeAudio RT deadline must be within its period");
        }
        Ok(())
    }
}

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

    scheduling_class: SchedulingClass,
    base_priority: u8,
    inherited_priority: u8,
    rt_period_ticks: u64,
    rt_budget_ticks: u64,
    rt_deadline_ticks: u64,
    rt_next_release_tick: u64,
    rt_absolute_deadline_tick: u64,
    rt_budget_remaining: u64,
    rt_job_active: bool,
    rt_deadline_miss_recorded: bool,
    deadline_misses: u64,
    budget_exhaustions: u64,

    preempt_guard_depth: u8,
    preempt_guard_start_tick: u64,
    preempt_guard_max_ticks: u64,
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
        scheduling_class: SchedulingClass::Normal,
        base_priority: 0,
        inherited_priority: 0,
        rt_period_ticks: 0,
        rt_budget_ticks: 0,
        rt_deadline_ticks: 0,
        rt_next_release_tick: 0,
        rt_absolute_deadline_tick: 0,
        rt_budget_remaining: 0,
        rt_job_active: false,
        rt_deadline_miss_recorded: false,
        deadline_misses: 0,
        budget_exhaustions: 0,
        preempt_guard_depth: 0,
        preempt_guard_start_tick: 0,
        preempt_guard_max_ticks: 0,
    };

    #[must_use]
    fn effective_priority(&self) -> u8 {
        core::cmp::max(self.base_priority, self.inherited_priority)
    }
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
    rt_dispatches: u64,
    preemption_deferrals: u64,
    guard_overruns: u64,
    account_global_ticks: bool,
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
            rt_dispatches: 0,
            preemption_deferrals: 0,
            guard_overruns: 0,
            account_global_ticks: true,
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
static AUDIO_RESERVED_CPU: AtomicUsize = AtomicUsize::new(NO_AUDIO_CPU);
static PRIORITY_INHERITANCE_EVENTS: AtomicU64 = AtomicU64::new(0);

static RT_PI_MUTEX: RtMutex = RtMutex::new(0x15_0000_0010);
static RT_PI_OWNER_OBSERVED_BOOST: AtomicBool = AtomicBool::new(false);
static RT_PI_WAITER_ACQUIRED: AtomicBool = AtomicBool::new(false);
static RT_AUDIO_JOBS_COMPLETED: AtomicU64 = AtomicU64::new(0);
static RT_AUDIO_TASK_FAILURE: AtomicBool = AtomicBool::new(false);

pub struct SchedulerReport {
    pub ticks: u64,
    pub preemptions: u64,
    pub tasks_completed: usize,
}

pub struct ForgeAudioRtReport {
    pub tick_hz: u32,
    pub ticks: u64,
    pub preemptions: u64,
    pub rt_dispatches: u64,
    pub tasks_completed: usize,
    pub audio_jobs_completed: u64,
    pub deadline_misses: u64,
    pub budget_exhaustions: u64,
    pub priority_inheritance_events: u64,
    pub preemption_deferrals: u64,
    pub guard_overruns: u64,
    pub audio_cpu_reserved: bool,
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
    let tasks_completed = finished_task_count();

    serial::println(format_args!(
        "[SCHED] Scheduler self-test complete: ticks={} preemptions={} workers={}",
        ticks, preemptions, tasks_completed
    ));

    reclaim_finished_task_stacks(allocator)?;

    Ok(SchedulerReport {
        ticks,
        preemptions,
        tasks_completed,
    })
}

pub fn run_forgeaudio_rt_self_test(
    allocator: &mut FrameAllocator<'_>,
    cpu_index: usize,
    calibrated_100hz_count: u32,
) -> Result<ForgeAudioRtReport, &'static str> {
    assert!(
        !x86_64::interrupts_enabled(),
        "ForgeAudio RT qualification requires interrupts disabled"
    );
    if calibrated_100hz_count < 10 {
        return Err("APIC calibration is too small for a 1 kHz ForgeAudio tick");
    }

    prepare_cpu_queue(cpu_index)?;
    unsafe { (*RUN_QUEUES[cpu_index].0.get()).account_global_ticks = false; }
    reserve_audio_cpu(cpu_index)?;

    PRIORITY_INHERITANCE_EVENTS.store(0, Ordering::Release);
    RT_PI_OWNER_OBSERVED_BOOST.store(false, Ordering::Release);
    RT_PI_WAITER_ACQUIRED.store(false, Ordering::Release);
    RT_AUDIO_JOBS_COMPLETED.store(0, Ordering::Release);
    RT_AUDIO_TASK_FAILURE.store(false, Ordering::Release);

    create_rt_task(
        allocator,
        cpu_index,
        rt_pi_owner_task,
        RtTaskConfig::audio(8, 128, 8, 16, 0),
    )?;
    create_rt_task(
        allocator,
        cpu_index,
        rt_pi_waiter_task,
        RtTaskConfig::audio(30, 128, 4, 16, 2),
    )?;
    create_rt_task(
        allocator,
        cpu_index,
        rt_audio_periodic_task,
        RtTaskConfig::audio(24, 4, 2, 3, 5),
    )?;
    create_task(allocator, cpu_index, rt_normal_load_task)?;

    let rt_timer_count = core::cmp::max(1, calibrated_100hz_count / 10);
    serial::println(format_args!(
        "[K15RT] ForgeAudio RT qualification start: cpu={} tick_hz={} apic_count={} reservation=true",
        cpu_index,
        FORGEAUDIO_RT_TICK_HZ,
        rt_timer_count,
    ));
    serial::println(format_args!(
        "[K15RT] policy: fixed-priority+deadline ordering budget_enforced=true PI=true bounded_preempt_guard=true affinity_cpu={}",
        cpu_index
    ));

    DEMO_COMPLETE.store(false, Ordering::Release);
    apic::start_periodic_timer(TIMER_VECTOR, rt_timer_count);
    x86_64::enable_interrupts();

    while !DEMO_COMPLETE.load(Ordering::Acquire) {
        halt_once();
    }

    x86_64::disable_interrupts();
    apic::mask_timer();

    let (ticks, preemptions, rt_dispatches, preemption_deferrals, guard_overruns) = unsafe {
        let queue = &*RUN_QUEUES[cpu_index].0.get();
        (
            queue.timer_ticks,
            queue.preemptions,
            queue.rt_dispatches,
            queue.preemption_deferrals,
            queue.guard_overruns,
        )
    };
    let tasks_completed = finished_task_count();
    let (deadline_misses, budget_exhaustions) = rt_failure_totals();
    let audio_jobs_completed = RT_AUDIO_JOBS_COMPLETED.load(Ordering::Acquire);
    let priority_inheritance_events = PRIORITY_INHERITANCE_EVENTS.load(Ordering::Acquire);
    let audio_cpu_reserved = AUDIO_RESERVED_CPU.load(Ordering::Acquire) == cpu_index;

    let report = ForgeAudioRtReport {
        tick_hz: FORGEAUDIO_RT_TICK_HZ,
        ticks,
        preemptions,
        rt_dispatches,
        tasks_completed,
        audio_jobs_completed,
        deadline_misses,
        budget_exhaustions,
        priority_inheritance_events,
        preemption_deferrals,
        guard_overruns,
        audio_cpu_reserved,
    };

    serial::println(format_args!(
        "[K15RT] result: ticks={} preemptions={} rt_dispatches={} tasks={} audio_jobs={} deadline_misses={} budget_exhaustions={} PI_events={} guard_deferrals={} guard_overruns={}",
        report.ticks,
        report.preemptions,
        report.rt_dispatches,
        report.tasks_completed,
        report.audio_jobs_completed,
        report.deadline_misses,
        report.budget_exhaustions,
        report.priority_inheritance_events,
        report.preemption_deferrals,
        report.guard_overruns,
    ));

    let qualification_ok = report.tasks_completed == 4
        && report.audio_jobs_completed == 8
        && report.deadline_misses == 0
        && report.budget_exhaustions == 0
        && report.priority_inheritance_events >= 1
        && RT_PI_OWNER_OBSERVED_BOOST.load(Ordering::Acquire)
        && RT_PI_WAITER_ACQUIRED.load(Ordering::Acquire)
        && report.preemption_deferrals >= 1
        && report.guard_overruns == 0
        && report.audio_cpu_reserved
        && !RT_AUDIO_TASK_FAILURE.load(Ordering::Acquire);

    if !qualification_ok {
        serial::emergency_println(format_args!(
            "[FAIL] K15.1 ForgeAudio RT qualification failed: owner_boost={} waiter_acquired={} audio_failure={}",
            RT_PI_OWNER_OBSERVED_BOOST.load(Ordering::Acquire),
            RT_PI_WAITER_ACQUIRED.load(Ordering::Acquire),
            RT_AUDIO_TASK_FAILURE.load(Ordering::Acquire),
        ));
        reclaim_finished_task_stacks(allocator)?;
        return Err("ForgeAudio RT execution qualification did not meet the stone contract");
    }

    serial::println(format_args!(
        "[K15OK] K15.1 ForgeAudio real-time execution foundation qualified: 1kHz tick, bounded budget, CPU affinity, priority inheritance, deadline tracking, preemption guard, audio reservation"
    ));

    reclaim_finished_task_stacks(allocator)?;
    prepare_cpu_queue(cpu_index)?;
    Ok(report)
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
            ..Task::EMPTY
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
    create_task_internal(allocator, cpu_index, entry, None)
}

fn create_rt_task(
    allocator: &mut FrameAllocator<'_>,
    cpu_index: usize,
    entry: extern "C" fn() -> !,
    config: RtTaskConfig,
) -> Result<TaskId, &'static str> {
    config.validate()?;
    if AUDIO_RESERVED_CPU.load(Ordering::Acquire) != cpu_index {
        return Err("ForgeAudio RT task requires the reserved audio CPU");
    }
    create_task_internal(allocator, cpu_index, entry, Some(config))
}

fn create_task_internal(
    allocator: &mut FrameAllocator<'_>,
    cpu_index: usize,
    entry: extern "C" fn() -> !,
    rt_config: Option<RtTaskConfig>,
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
            let _ = allocator.deallocate_contiguous(stack_base, TASK_STACK_PAGES);
            return Err("kernel task table is full");
        }

        let task_id = table.next_id;
        table.next_id = table.next_id.checked_add(1).ok_or("task ID overflow")?;
        let task_index = table.count;

        let mut task = Task {
            id: task_id,
            state: TaskState::Ready,
            saved_frame,
            stack_base,
            affinity_cpu: cpu_index,
            wait_object: 0,
            runtime_ticks: 0,
            started: false,
            ..Task::EMPTY
        };

        if let Some(config) = rt_config {
            task.scheduling_class = SchedulingClass::RealtimeAudio;
            task.base_priority = config.priority;
            task.rt_period_ticks = config.period_ticks;
            task.rt_budget_ticks = config.budget_ticks;
            task.rt_deadline_ticks = config.deadline_ticks;
            task.rt_next_release_tick = if config.release_offset_ticks == 0 {
                config.period_ticks
            } else {
                config.release_offset_ticks
            };
            if config.release_offset_ticks == 0 {
                task.rt_job_active = true;
                task.rt_budget_remaining = config.budget_ticks;
                task.rt_absolute_deadline_tick = config.deadline_ticks;
            } else {
                task.state = TaskState::Blocked;
                task.wait_object = RT_PERIOD_WAIT_OBJECT;
            }
        }

        table.tasks[task_index] = task;
        table.count += 1;

        let queue = &mut *RUN_QUEUES[cpu_index].0.get();
        if let Err(error) = queue.push(task_index) {
            table.count -= 1;
            table.tasks[task_index] = Task::EMPTY;
            let _ = allocator.deallocate_contiguous(stack_base, TASK_STACK_PAGES);
            return Err(error);
        }
        Ok(task_id)
    }
}

fn build_initial_frame(stack_top: u64, entry: u64) -> Result<u64, &'static str> {
    if size_of::<InterruptFrame>() != 22 * size_of::<u64>() {
        return Err("interrupt-frame layout changed unexpectedly");
    }

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
        let mut current_budget_exhausted = false;
        if timer_interrupt {
            queue.timer_ticks = queue.timer_ticks.saturating_add(1);
            if queue.account_global_ticks {
                percpu::increment_tick(cpu_index);
            }
            release_due_rt_jobs(table, queue.timer_ticks, cpu_index);
        }

        {
            let current = &mut table.tasks[current_index];
            current.saved_frame = frame as u64;

            if timer_interrupt {
                current.runtime_ticks = current.runtime_ticks.saturating_add(1);
                if current.scheduling_class == SchedulingClass::RealtimeAudio && current.rt_job_active {
                    if !current.rt_deadline_miss_recorded
                        && queue.timer_ticks > current.rt_absolute_deadline_tick
                    {
                        current.deadline_misses = current.deadline_misses.saturating_add(1);
                        current.rt_deadline_miss_recorded = true;
                    }
                    if current.id != IDLE_TASK_ID && current.rt_budget_remaining > 0 {
                        current.rt_budget_remaining -= 1;
                        if current.rt_budget_remaining == 0 {
                            current.budget_exhaustions = current.budget_exhaustions.saturating_add(1);
                            current.rt_job_active = false;
                            current.wait_object = RT_PERIOD_WAIT_OBJECT;
                            current.state = TaskState::Blocked;
                            current_budget_exhausted = true;
                        }
                    }
                }
            }
        }

        if timer_interrupt {
            let current = &mut table.tasks[current_index];
            if !current_budget_exhausted && current.preempt_guard_depth != 0 {
                let age = queue.timer_ticks.saturating_sub(current.preempt_guard_start_tick);
                if age <= current.preempt_guard_max_ticks {
                    queue.preemption_deferrals = queue.preemption_deferrals.saturating_add(1);
                    apic::end_of_interrupt();
                    return frame;
                }
                queue.guard_overruns = queue.guard_overruns.saturating_add(1);
                current.preempt_guard_depth = 0;
                serial::emergency_println(format_args!(
                    "[K15RT] bounded preemption guard overrun: task={} age_ticks={} max_ticks={}",
                    current.id, age, current.preempt_guard_max_ticks
                ));
            }
            apic::end_of_interrupt();
        }

        if table.tasks[current_index].state == TaskState::Running {
            table.tasks[current_index].state = TaskState::Ready;
        }

        let (next_position, next_index) = choose_next(table, queue)
            .unwrap_or((queue.cursor, current_index));
        if !matches!(
            table.tasks[next_index].state,
            TaskState::Ready | TaskState::Running
        ) {
            table.tasks[current_index].state = TaskState::Running;
            return frame;
        }

        if timer_interrupt && next_index != current_index {
            queue.preemptions = queue.preemptions.saturating_add(1);
        }

        let next = &mut table.tasks[next_index];
        if next.scheduling_class == SchedulingClass::RealtimeAudio {
            queue.rt_dispatches = queue.rt_dispatches.saturating_add(1);
        }
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

fn release_due_rt_jobs(table: &mut TaskTable, now: u64, cpu_index: usize) {
    for task in &mut table.tasks[..table.count] {
        if task.scheduling_class != SchedulingClass::RealtimeAudio
            || task.affinity_cpu != cpu_index
            || task.state == TaskState::Finished
            || task.rt_period_ticks == 0
        {
            continue;
        }

        while now >= task.rt_next_release_tick {
            if task.rt_job_active && !task.rt_deadline_miss_recorded {
                task.deadline_misses = task.deadline_misses.saturating_add(1);
            }
            let release_tick = task.rt_next_release_tick;
            task.rt_next_release_tick = task
                .rt_next_release_tick
                .saturating_add(task.rt_period_ticks);
            task.rt_absolute_deadline_tick = release_tick.saturating_add(task.rt_deadline_ticks);
            task.rt_budget_remaining = task.rt_budget_ticks;
            task.rt_job_active = true;
            task.rt_deadline_miss_recorded = false;
            if task.state == TaskState::Blocked && task.wait_object == RT_PERIOD_WAIT_OBJECT {
                task.wait_object = 0;
                task.state = TaskState::Ready;
            }
        }
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
    let cpu_index = percpu::bsp_index();
    let mut best_rt: Option<(usize, usize, u8, u64)> = None;

    for position in 0..queue.len {
        let task_index = queue.entries[position];
        let task = table.tasks[task_index];
        if task.affinity_cpu != cpu_index
            || task.scheduling_class != SchedulingClass::RealtimeAudio
            || !task.rt_job_active
            || task.rt_budget_remaining == 0
            || !matches!(task.state, TaskState::Ready | TaskState::Running)
        {
            continue;
        }

        let priority = task.effective_priority();
        let deadline = task.rt_absolute_deadline_tick;
        match best_rt {
            None => best_rt = Some((position, task_index, priority, deadline)),
            Some((_, _, best_priority, best_deadline))
                if priority > best_priority
                    || (priority == best_priority && deadline < best_deadline) =>
            {
                best_rt = Some((position, task_index, priority, deadline));
            }
            _ => {}
        }
    }

    if let Some((position, task_index, _, _)) = best_rt {
        return Some((position, task_index));
    }

    for offset in 1..=queue.len {
        let position = (queue.cursor + offset) % queue.len;
        let task_index = queue.entries[position];
        let task = table.tasks[task_index];
        if task.affinity_cpu == cpu_index
            && task.scheduling_class == SchedulingClass::Normal
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

#[must_use]
pub fn current_effective_priority() -> u8 {
    let cpu_index = percpu::bsp_index();
    unsafe {
        let queue = &*RUN_QUEUES[cpu_index].0.get();
        let table = &*TASK_TABLE.0.get();
        table.tasks[queue.current_task_index].effective_priority()
    }
}

#[must_use]
pub fn task_effective_priority(task_id: TaskId) -> Option<u8> {
    unsafe {
        let table = &*TASK_TABLE.0.get();
        table.tasks[..table.count]
            .iter()
            .find(|task| task.id == task_id)
            .map(Task::effective_priority)
    }
}

pub fn set_inherited_priority(task_id: TaskId, priority: u8) {
    assert!(
        !x86_64::interrupts_enabled(),
        "priority inheritance update requires interrupts disabled"
    );
    unsafe {
        let table = &mut *TASK_TABLE.0.get();
        if let Some(task) = table.tasks[..table.count]
            .iter_mut()
            .find(|task| task.id == task_id)
        {
            let bounded = core::cmp::min(priority, FORGEAUDIO_RT_MAX_PRIORITY);
            if bounded > task.inherited_priority {
                PRIORITY_INHERITANCE_EVENTS.fetch_add(1, Ordering::AcqRel);
            }
            task.inherited_priority = bounded;
        }
    }
}

pub fn clear_inherited_priority(task_id: TaskId) {
    assert!(
        !x86_64::interrupts_enabled(),
        "priority inheritance clear requires interrupts disabled"
    );
    unsafe {
        let table = &mut *TASK_TABLE.0.get();
        if let Some(task) = table.tasks[..table.count]
            .iter_mut()
            .find(|task| task.id == task_id)
        {
            task.inherited_priority = 0;
        }
    }
}

pub fn reserve_audio_cpu(cpu_index: usize) -> Result<(), &'static str> {
    if cpu_index >= MAX_CPUS {
        return Err("ForgeAudio CPU reservation outside logical CPU table");
    }
    match AUDIO_RESERVED_CPU.compare_exchange(
        NO_AUDIO_CPU,
        cpu_index,
        Ordering::AcqRel,
        Ordering::Acquire,
    ) {
        Ok(_) => Ok(()),
        Err(existing) if existing == cpu_index => Ok(()),
        Err(_) => Err("ForgeAudio audio execution CPU already reserved elsewhere"),
    }
}

#[must_use]
pub fn audio_reserved_cpu() -> Option<usize> {
    let cpu = AUDIO_RESERVED_CPU.load(Ordering::Acquire);
    if cpu == NO_AUDIO_CPU { None } else { Some(cpu) }
}

pub fn enter_preemption_guard(max_ticks: u64) -> Result<(), &'static str> {
    if max_ticks == 0 {
        return Err("preemption guard bound must be nonzero");
    }
    let saved_flags = x86_64::save_and_disable_interrupts();
    let cpu_index = percpu::bsp_index();
    unsafe {
        let queue = &*RUN_QUEUES[cpu_index].0.get();
        let table = &mut *TASK_TABLE.0.get();
        let task = &mut table.tasks[queue.current_task_index];
        if task.id == IDLE_TASK_ID {
            x86_64::restore_interrupts(saved_flags);
            return Err("idle task cannot enter a preemption guard");
        }
        if task.preempt_guard_depth == u8::MAX {
            x86_64::restore_interrupts(saved_flags);
            return Err("preemption guard nesting overflow");
        }
        if task.preempt_guard_depth == 0 {
            task.preempt_guard_start_tick = queue.timer_ticks;
            task.preempt_guard_max_ticks = max_ticks;
        } else {
            task.preempt_guard_max_ticks = core::cmp::min(task.preempt_guard_max_ticks, max_ticks);
        }
        task.preempt_guard_depth += 1;
    }
    x86_64::restore_interrupts(saved_flags);
    Ok(())
}

pub fn exit_preemption_guard() -> Result<(), &'static str> {
    let saved_flags = x86_64::save_and_disable_interrupts();
    let cpu_index = percpu::bsp_index();
    let result = unsafe {
        let queue = &*RUN_QUEUES[cpu_index].0.get();
        let table = &mut *TASK_TABLE.0.get();
        let task = &mut table.tasks[queue.current_task_index];
        if task.preempt_guard_depth == 0 {
            Err("preemption guard underflow")
        } else {
            task.preempt_guard_depth -= 1;
            if task.preempt_guard_depth == 0 {
                task.preempt_guard_start_tick = 0;
                task.preempt_guard_max_ticks = 0;
            }
            Ok(())
        }
    };
    x86_64::restore_interrupts(saved_flags);
    result
}

#[must_use]
pub fn current_rt_clock_tick() -> u64 {
    let cpu_index = percpu::bsp_index();
    unsafe { (*RUN_QUEUES[cpu_index].0.get()).timer_ticks }
}

#[must_use]
pub fn current_rt_deadline_tick() -> Option<u64> {
    let cpu_index = percpu::bsp_index();
    unsafe {
        let queue = &*RUN_QUEUES[cpu_index].0.get();
        let table = &*TASK_TABLE.0.get();
        let task = &table.tasks[queue.current_task_index];
        if task.scheduling_class == SchedulingClass::RealtimeAudio && task.rt_job_active {
            Some(task.rt_absolute_deadline_tick)
        } else {
            None
        }
    }
}

pub fn wait_until_next_rt_period() -> Result<(), &'static str> {
    let saved_flags = x86_64::save_and_disable_interrupts();
    let cpu_index = percpu::bsp_index();
    let result = unsafe {
        let queue = &*RUN_QUEUES[cpu_index].0.get();
        let table = &mut *TASK_TABLE.0.get();
        let task = &mut table.tasks[queue.current_task_index];
        if task.scheduling_class != SchedulingClass::RealtimeAudio {
            Err("normal task cannot wait on an RT period")
        } else if !task.rt_job_active {
            Err("RT task has no active job to complete")
        } else {
            task.rt_job_active = false;
            task.wait_object = RT_PERIOD_WAIT_OBJECT;
            task.state = TaskState::Blocked;
            Ok(())
        }
    };
    if result.is_ok() {
        reschedule_blocked_current();
    }
    x86_64::restore_interrupts(saved_flags);
    result
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

pub fn reschedule_current() {
    reschedule_blocked_current();
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
        current.rt_job_active = false;
        current.preempt_guard_depth = 0;

        let complete = table.tasks[1..table.count]
            .iter()
            .all(|task| task.state == TaskState::Finished);
        if complete {
            DEMO_COMPLETE.store(true, Ordering::Release);
        }
    }

    let _ = saved_flags;
    reschedule_blocked_current();
    halt_forever();
}

fn finished_task_count() -> usize {
    unsafe {
        let table = &*TASK_TABLE.0.get();
        table.tasks[1..table.count]
            .iter()
            .filter(|task| task.state == TaskState::Finished)
            .count()
    }
}

fn rt_failure_totals() -> (u64, u64) {
    unsafe {
        let table = &*TASK_TABLE.0.get();
        table.tasks[1..table.count]
            .iter()
            .filter(|task| task.scheduling_class == SchedulingClass::RealtimeAudio)
            .fold((0, 0), |(misses, exhausted), task| {
                (
                    misses.saturating_add(task.deadline_misses),
                    exhausted.saturating_add(task.budget_exhaustions),
                )
            })
    }
}

fn reclaim_finished_task_stacks(
    allocator: &mut FrameAllocator<'_>,
) -> Result<(), &'static str> {
    unsafe {
        let table = &mut *TASK_TABLE.0.get();
        for task in &mut table.tasks[1..table.count] {
            if task.state == TaskState::Finished && task.stack_base != 0 {
                allocator.deallocate_contiguous(task.stack_base, TASK_STACK_PAGES)?;
                task.stack_base = 0;
                task.saved_frame = 0;
            }
        }
    }
    Ok(())
}

fn wait_ticks(delta: u64) {
    let cpu_index = percpu::bsp_index();
    let target = percpu::ticks(cpu_index).saturating_add(delta);
    while percpu::ticks(cpu_index) < target {
        pause();
    }
}

fn wait_rt_ticks(delta: u64) {
    let target = current_rt_clock_tick().saturating_add(delta);
    while current_rt_clock_tick() < target {
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

extern "C" fn rt_pi_owner_task() -> ! {
    if RT_PI_MUTEX.lock().is_err() {
        RT_AUDIO_TASK_FAILURE.store(true, Ordering::Release);
        finish_current();
    }
    serial::println(format_args!("[K15RT] PI owner acquired mutex at tick={}", current_rt_clock_tick()));

    if enter_preemption_guard(1).is_err() {
        RT_AUDIO_TASK_FAILURE.store(true, Ordering::Release);
        let _ = RT_PI_MUTEX.unlock();
        finish_current();
    }
    wait_rt_ticks(1);
    if exit_preemption_guard().is_err() {
        RT_AUDIO_TASK_FAILURE.store(true, Ordering::Release);
    }

    wait_rt_ticks(2);
    if current_effective_priority() >= 30 {
        RT_PI_OWNER_OBSERVED_BOOST.store(true, Ordering::Release);
        serial::println(format_args!(
            "[K15RT] PI boost observed: owner={} effective_priority={}",
            current_task_id(),
            current_effective_priority()
        ));
    } else {
        RT_AUDIO_TASK_FAILURE.store(true, Ordering::Release);
    }

    if RT_PI_MUTEX.unlock().is_err() {
        RT_AUDIO_TASK_FAILURE.store(true, Ordering::Release);
    }
    finish_current();
}

extern "C" fn rt_pi_waiter_task() -> ! {
    serial::println(format_args!(
        "[K15RT] PI waiter released: task={} priority={} tick={}",
        current_task_id(),
        current_effective_priority(),
        current_rt_clock_tick()
    ));
    if RT_PI_MUTEX.lock().is_err() {
        RT_AUDIO_TASK_FAILURE.store(true, Ordering::Release);
        finish_current();
    }
    RT_PI_WAITER_ACQUIRED.store(true, Ordering::Release);
    serial::println(format_args!(
        "[K15RT] PI waiter acquired mutex after ownership transfer at tick={}",
        current_rt_clock_tick()
    ));
    if RT_PI_MUTEX.unlock().is_err() {
        RT_AUDIO_TASK_FAILURE.store(true, Ordering::Release);
    }
    finish_current();
}

extern "C" fn rt_audio_periodic_task() -> ! {
    for job in 0..8u64 {
        let start = current_rt_clock_tick();
        let Some(deadline) = current_rt_deadline_tick() else {
            RT_AUDIO_TASK_FAILURE.store(true, Ordering::Release);
            finish_current();
        };
        if start > deadline {
            RT_AUDIO_TASK_FAILURE.store(true, Ordering::Release);
        }

        wait_rt_ticks(1);
        RT_AUDIO_JOBS_COMPLETED.fetch_add(1, Ordering::AcqRel);
        serial::println(format_args!(
            "[K15RT] audio job={} start={} finish={} deadline={} budget_ok=true",
            job + 1,
            start,
            current_rt_clock_tick(),
            deadline,
        ));

        if job != 7 && wait_until_next_rt_period().is_err() {
            RT_AUDIO_TASK_FAILURE.store(true, Ordering::Release);
            finish_current();
        }
    }
    finish_current();
}

extern "C" fn rt_normal_load_task() -> ! {
    let start = current_rt_clock_tick();
    wait_rt_ticks(45);
    let elapsed = current_rt_clock_tick().saturating_sub(start);
    serial::println(format_args!(
        "[K15RT] competing normal workload completed: elapsed_ticks={} audio_jobs={}",
        elapsed,
        RT_AUDIO_JOBS_COMPLETED.load(Ordering::Acquire)
    ));
    finish_current();
}
