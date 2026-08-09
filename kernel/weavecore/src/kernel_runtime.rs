//! Live K11.0 runtime services shared by processes and ForgeBus.
use crate::{
    block_queue::BlockRequestQueue,
    driver_watchdog::DriverWatchdog,
    interrupt_router::InterruptRouter,
    object_lifecycle::ObjectLifecycle,
    sync::SpinLock,
};

pub struct KernelRuntime {
    pub objects: ObjectLifecycle,
    pub block_requests: BlockRequestQueue,
    pub interrupts: InterruptRouter,
    pub watchdog: DriverWatchdog,
}
impl KernelRuntime {
    pub const fn new() -> Self {
        Self {
            objects: ObjectLifecycle::new(),
            block_requests: BlockRequestQueue::new(),
            interrupts: InterruptRouter::new(),
            watchdog: DriverWatchdog::new(),
        }
    }
}

static RUNTIME: SpinLock<KernelRuntime> = SpinLock::new(KernelRuntime::new());

pub fn with_runtime<R>(operation: impl FnOnce(&mut KernelRuntime) -> R) -> R {
    let mut runtime = RUNTIME.lock();
    operation(&mut runtime)
}
