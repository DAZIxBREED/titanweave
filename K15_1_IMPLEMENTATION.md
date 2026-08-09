# K15.1 — ForgeAudio Real-Time Audio Execution Foundation

K15.1 is the first gate of the locked 16-gate ForgeAudio stone contract. It is built additively on the frozen K14.C32 baseline and does not introduce an audio device or userspace audio ABI yet; those are K15.2-K15.4.

## Implemented

### Real-time scheduling class
`kernel/weavecore/src/scheduler.rs` now contains a real `SchedulingClass::RealtimeAudio` path with fixed-priority selection and absolute-deadline tie breaking. Normal tasks remain round-robin when no eligible RT task is runnable.

### Period, deadline and execution budget
Each RT task owns explicit period, budget, relative deadline, next-release, absolute-deadline and remaining-budget state. The 1 kHz qualification tick releases periodic work, records deadline misses and forcibly blocks a task whose budget is exhausted until its next release.

### CPU affinity and ForgeAudio reservation
RT tasks retain hard CPU affinity and may only be created on the CPU reserved through `reserve_audio_cpu`. K15.1 reserves the BSP execution CPU during qualification because the current kernel-task scheduler is BSP-owned; the reservation is retained as a real scheduler resource for later ForgeAudio gates.

### Priority inheritance
`kernel/weavecore/src/rt_mutex.rs` implements a bounded sleepable real-time mutex. A contending task blocks rather than spins, donates its effective priority to the owner, and ownership transfers directly to the highest-priority waiter on unlock.

### Bounded preemption guard
RT/kernel tasks can enter a nested preemption guard with an explicit tick bound. A timer interrupt may defer scheduling while the guard is within that bound; an overrun is recorded and the scheduler forcibly drops the guard instead of allowing unbounded preemption suppression.

### Runtime qualification workload
K15.1 boots a 1 kHz scheduler qualification workload before normal K14 graphics/device initialization:

- a low-priority RT task owns a priority-inheriting mutex;
- a high-priority RT waiter is released later and blocks on that mutex;
- the owner must observe the inherited priority before releasing it;
- the waiter must receive ownership;
- the owner exercises one bounded preemption deferral;
- an independent periodic ForgeAudio-style task executes eight jobs at a 4 ms period with a 2 ms budget and 3 ms relative deadline;
- a competing normal workload remains runnable during the test;
- all RT jobs must complete with zero deadline misses, zero budget exhaustions and zero preemption-guard overruns.

The test reclaims all temporary task stacks before continuing boot.

## Required runtime marker

A passing system emits:

```text
[K15OK] K15.1 ForgeAudio real-time execution foundation qualified: 1kHz tick, bounded budget, CPU affinity, priority inheritance, deadline tracking, preemption guard, audio reservation
```

A source-only build is **not** runtime-qualified until the QEMU harness observes that marker and the inherited K14.C32 qualification path reaches intentional HALT with no `[FAIL]` lines.
