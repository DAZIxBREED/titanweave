//! K15.1 priority-inheriting sleepable mutex for real-time kernel work.
//!
//! This is intentionally separate from the interrupt-safe `SpinLock`: a real-
//! time task that cannot immediately obtain an `RtMutex` blocks and donates its
//! effective priority to the current owner instead of spinning through an
//! audio deadline. Ownership is transferred directly to the highest-priority
//! waiter on unlock.

use crate::arch::x86_64;
use crate::objects::ObjectId;
use crate::scheduler::{self, TaskId};
use crate::sync::SpinLock;

const MAX_RT_MUTEX_WAITERS: usize = 16;

#[derive(Clone, Copy)]
struct Waiter {
    task: TaskId,
    priority: u8,
}

impl Waiter {
    const EMPTY: Self = Self { task: 0, priority: 0 };
}

struct RtMutexInner {
    owner: TaskId,
    waiters: [Waiter; MAX_RT_MUTEX_WAITERS],
    waiter_count: usize,
}

impl RtMutexInner {
    const fn new() -> Self {
        Self {
            owner: 0,
            waiters: [Waiter::EMPTY; MAX_RT_MUTEX_WAITERS],
            waiter_count: 0,
        }
    }

    fn push_waiter(&mut self, waiter: Waiter) -> Result<(), &'static str> {
        if self.waiters[..self.waiter_count]
            .iter()
            .any(|entry| entry.task == waiter.task)
        {
            return Ok(());
        }
        if self.waiter_count == MAX_RT_MUTEX_WAITERS {
            return Err("RT mutex waiter table is full");
        }
        self.waiters[self.waiter_count] = waiter;
        self.waiter_count += 1;
        Ok(())
    }

    fn highest_waiter_index(&self) -> Option<usize> {
        let mut best: Option<(usize, u8)> = None;
        for (index, waiter) in self.waiters[..self.waiter_count].iter().enumerate() {
            match best {
                None => best = Some((index, waiter.priority)),
                Some((_, priority)) if waiter.priority > priority => {
                    best = Some((index, waiter.priority));
                }
                _ => {}
            }
        }
        best.map(|(index, _)| index)
    }

    fn pop_highest_waiter(&mut self) -> Option<Waiter> {
        let index = self.highest_waiter_index()?;
        let waiter = self.waiters[index];
        for current in (index + 1)..self.waiter_count {
            self.waiters[current - 1] = self.waiters[current];
        }
        self.waiter_count -= 1;
        self.waiters[self.waiter_count] = Waiter::EMPTY;
        Some(waiter)
    }

    fn highest_waiter_priority(&self) -> u8 {
        self.waiters[..self.waiter_count]
            .iter()
            .map(|waiter| waiter.priority)
            .max()
            .unwrap_or(0)
    }
}

pub struct RtMutex {
    object_id: ObjectId,
    inner: SpinLock<RtMutexInner>,
}

impl RtMutex {
    pub const fn new(object_id: ObjectId) -> Self {
        Self {
            object_id,
            inner: SpinLock::new(RtMutexInner::new()),
        }
    }

    pub fn lock(&self) -> Result<(), &'static str> {
        let saved_flags = x86_64::save_and_disable_interrupts();
        let current = scheduler::current_task_id();
        if current == 0 {
            x86_64::restore_interrupts(saved_flags);
            return Err("idle task cannot own an RT mutex");
        }

        {
            let mut inner = self.inner.lock();
            if inner.owner == 0 {
                inner.owner = current;
                drop(inner);
                x86_64::restore_interrupts(saved_flags);
                return Ok(());
            }
            if inner.owner == current {
                drop(inner);
                x86_64::restore_interrupts(saved_flags);
                return Err("recursive RT mutex acquisition is not supported");
            }

            let priority = scheduler::current_effective_priority();
            if let Err(error) = inner.push_waiter(Waiter { task: current, priority }) {
                drop(inner);
                x86_64::restore_interrupts(saved_flags);
                return Err(error);
            }
            let inherited = inner.highest_waiter_priority();
            scheduler::set_inherited_priority(inner.owner, inherited);
            scheduler::prepare_block_current(self.object_id);
        }

        scheduler::reschedule_blocked_current();

        let owns_after_wake = {
            let inner = self.inner.lock();
            inner.owner == current
        };
        x86_64::restore_interrupts(saved_flags);
        if owns_after_wake {
            Ok(())
        } else {
            Err("RT mutex wake occurred without ownership transfer")
        }
    }

    pub fn unlock(&self) -> Result<(), &'static str> {
        let saved_flags = x86_64::save_and_disable_interrupts();
        let current = scheduler::current_task_id();
        let mut should_reschedule = false;

        {
            let mut inner = self.inner.lock();
            if inner.owner != current {
                drop(inner);
                x86_64::restore_interrupts(saved_flags);
                return Err("RT mutex unlock attempted by non-owner");
            }

            scheduler::clear_inherited_priority(current);
            if let Some(next) = inner.pop_highest_waiter() {
                inner.owner = next.task;
                let remaining_priority = inner.highest_waiter_priority();
                if remaining_priority != 0 {
                    scheduler::set_inherited_priority(next.task, remaining_priority);
                }
                scheduler::wake_task(next.task);
                should_reschedule = next.priority > scheduler::current_effective_priority();
            } else {
                inner.owner = 0;
            }
        }

        if should_reschedule {
            scheduler::reschedule_current();
        }
        x86_64::restore_interrupts(saved_flags);
        Ok(())
    }
}
