use crate::acpi::MAX_CPUS;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

pub struct CpuLocal {
    online: AtomicBool,
    logical_index: AtomicUsize,
    apic_id: AtomicU32,
    current_task: AtomicU32,
    scheduler_ticks: AtomicU64,
}

impl CpuLocal {
    const fn new() -> Self {
        Self {
            online: AtomicBool::new(false),
            logical_index: AtomicUsize::new(usize::MAX),
            apic_id: AtomicU32::new(u32::MAX),
            current_task: AtomicU32::new(0),
            scheduler_ticks: AtomicU64::new(0),
        }
    }
}

static CPUS: [CpuLocal; MAX_CPUS] = [const { CpuLocal::new() }; MAX_CPUS];
static BSP_LOGICAL_INDEX: AtomicUsize = AtomicUsize::new(0);

pub fn initialize(cpu_index: usize, apic_id: u32, is_bsp: bool) {
    assert!(cpu_index < MAX_CPUS, "per-CPU index outside static table");
    let cpu = &CPUS[cpu_index];
    cpu.logical_index.store(cpu_index, Ordering::Release);
    cpu.apic_id.store(apic_id, Ordering::Release);
    cpu.online.store(true, Ordering::Release);
    cpu.current_task.store(0, Ordering::Release);
    cpu.scheduler_ticks.store(0, Ordering::Release);
    if is_bsp {
        BSP_LOGICAL_INDEX.store(cpu_index, Ordering::Release);
    }
}

#[must_use]
pub fn bsp_index() -> usize {
    BSP_LOGICAL_INDEX.load(Ordering::Acquire)
}

#[must_use]
pub fn apic_id(cpu_index: usize) -> Option<u32> {
    if cpu_index >= MAX_CPUS || !CPUS[cpu_index].online.load(Ordering::Acquire) {
        return None;
    }
    let value = CPUS[cpu_index].apic_id.load(Ordering::Acquire);
    if value == u32::MAX { None } else { Some(value) }
}

pub fn set_current_task(cpu_index: usize, task_id: u32) {
    CPUS[cpu_index]
        .current_task
        .store(task_id, Ordering::Release);
}

#[must_use]
pub fn current_task(cpu_index: usize) -> u32 {
    CPUS[cpu_index].current_task.load(Ordering::Acquire)
}

pub fn increment_tick(cpu_index: usize) -> u64 {
    CPUS[cpu_index]
        .scheduler_ticks
        .fetch_add(1, Ordering::AcqRel)
        + 1
}

#[must_use]
pub fn ticks(cpu_index: usize) -> u64 {
    CPUS[cpu_index].scheduler_ticks.load(Ordering::Acquire)
}

#[must_use]
pub fn online_count() -> usize {
    CPUS.iter()
        .filter(|cpu| cpu.online.load(Ordering::Acquire))
        .count()
}
