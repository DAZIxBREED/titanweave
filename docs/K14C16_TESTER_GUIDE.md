# K14.C16 Tester Guide

Run source validation, build, then the QEMU runtime qualification. QEMU has no native Radeon, so the reviewed MMIO write must be reported as safely deferred.

Expected markers include `C16RV`, `C16PG`, `C16HW`, `C16RD`, `C16OK`, the DISPLAYD C16 gate message, stable userspace handoff, the K14.C16 alive marker, qualification marker, and intentional halt.

A QEMU PASS qualifies the runtime policy/deferred path only. It does not constitute a physical Radeon MMIO-write qualification.
