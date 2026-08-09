use crate::arch::x86_64;
use crate::scheduler::{self, TaskId};
use crate::sync::SpinLock;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub type ObjectId = u64;
const MAX_WAITERS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum ObjectKind {
    Event = 1,
    Process = 2,
    AddressSpace = 3,
    Channel = 4,
    Console = 5,
}

pub struct ObjectHeader {
    id: ObjectId,
    kind: ObjectKind,
    references: AtomicU32,
}

impl ObjectHeader {
    pub const fn new_static(id: ObjectId, kind: ObjectKind) -> Self {
        Self {
            id,
            kind,
            references: AtomicU32::new(1),
        }
    }

    #[must_use]
    pub const fn id(&self) -> ObjectId {
        self.id
    }

    #[must_use]
    pub const fn kind(&self) -> ObjectKind {
        self.kind
    }

    pub fn retain(&self) -> u32 {
        self.references.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn release(&self) -> u32 {
        let previous = self.references.fetch_sub(1, Ordering::AcqRel);
        assert!(previous != 0, "kernel object reference underflow");
        previous - 1
    }

    #[must_use]
    pub fn reference_count(&self) -> u32 {
        self.references.load(Ordering::Acquire)
    }
}

#[derive(Clone, Copy)]
struct WaitQueueInner {
    tasks: [TaskId; MAX_WAITERS],
    count: usize,
}

impl WaitQueueInner {
    const fn new() -> Self {
        Self {
            tasks: [0; MAX_WAITERS],
            count: 0,
        }
    }

    fn push(&mut self, task_id: TaskId) -> Result<(), &'static str> {
        if self.tasks[..self.count].contains(&task_id) {
            return Ok(());
        }
        if self.count == MAX_WAITERS {
            return Err("kernel wait queue is full");
        }
        self.tasks[self.count] = task_id;
        self.count += 1;
        Ok(())
    }

    fn pop_front(&mut self) -> Option<TaskId> {
        if self.count == 0 {
            return None;
        }
        let task_id = self.tasks[0];
        for index in 1..self.count {
            self.tasks[index - 1] = self.tasks[index];
        }
        self.count -= 1;
        self.tasks[self.count] = 0;
        Some(task_id)
    }
}

pub struct WaitQueue {
    inner: SpinLock<WaitQueueInner>,
}

impl WaitQueue {
    pub const fn new() -> Self {
        Self {
            inner: SpinLock::new(WaitQueueInner::new()),
        }
    }
}

/// First blocking kernel object. K3 events are auto-reset: one signal releases
/// one waiter, or remains latched until the next waiter consumes it.
pub struct KernelEvent {
    header: ObjectHeader,
    signaled: AtomicBool,
    waiters: WaitQueue,
}

impl KernelEvent {
    pub const fn new_static(id: ObjectId, initially_signaled: bool) -> Self {
        Self {
            header: ObjectHeader::new_static(id, ObjectKind::Event),
            signaled: AtomicBool::new(initially_signaled),
            waiters: WaitQueue::new(),
        }
    }

    #[must_use]
    pub const fn header(&self) -> &ObjectHeader {
        &self.header
    }

    pub fn wait(&self) -> Result<(), &'static str> {
        let saved_flags = x86_64::save_and_disable_interrupts();
        let current = scheduler::current_task_id();

        {
            let mut queue = self.waiters.inner.lock();
            if self.signaled.swap(false, Ordering::AcqRel) {
                drop(queue);
                x86_64::restore_interrupts(saved_flags);
                return Ok(());
            }

            if let Err(error) = queue.push(current) {
                drop(queue);
                x86_64::restore_interrupts(saved_flags);
                return Err(error);
            }
            // Mark the task blocked before releasing the queue lock. A signal
            // from another CPU can then safely find and wake it without a
            // missed-wakeup window.
            scheduler::prepare_block_current(self.header.id());
        }

        scheduler::reschedule_blocked_current();
        x86_64::restore_interrupts(saved_flags);
        Ok(())
    }

    pub fn signal(&self) {
        let mut queue = self.waiters.inner.lock();
        if let Some(task_id) = queue.pop_front() {
            scheduler::wake_task(task_id);
        } else {
            self.signaled.store(true, Ordering::Release);
        }
    }
}

pub static DEMO_EVENT: KernelEvent = KernelEvent::new_static(1, false);
