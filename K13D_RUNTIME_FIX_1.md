# K13.D Runtime Fix 1 — Deterministic DISPLAYD Qualification Handshake

The first K13.D QEMU run proved all kernel resilience/multi-GPU milestones but
missed the DISPLAYD present/recovery success banners. The scripted shell exited
while DISPLAYD was returning from a successful `SYS_GPU_PRESENT`, and the K11-era
qualification boundary immediately terminated persistent services.

This fix makes qualification depend on two independent facts:

1. the scripted shell exits successfully; and
2. DISPLAYD completes `SYS_GPU_RECOVER` and performs the following userspace write.

The recovery syscall records a pending success/failure result. The next successful
DISPLAYD `SYS_WRITE` acknowledges that userspace observed the result. Only after
both the shell and DISPLAYD acknowledgement exist can the qualification teardown
run. This avoids matching banner text or process names and does not add a test-only
syscall to the ABI.
