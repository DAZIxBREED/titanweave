# K11 Runtime Fix 7 — Qualification Completion Boundary

The K11 native-service test intentionally includes persistent service stubs that remain in yield loops.
That made the old `all_terminal()` completion condition impossible after the scripted shell finished, so
the QEMU qualification harness could never observe the final K11 completion markers.

Fix: a clean exit of the scripted Shell service, with no prior user faults, is now the explicit K11
qualification boundary. The kernel masks the timer, returns to the kernel qualification continuation,
reclaims all remaining process resources, emits the final K11 markers, and halts intentionally.

This preserves persistent service behavior during normal test execution rather than converting daemon
stubs into one-shot programs merely to satisfy the harness.
