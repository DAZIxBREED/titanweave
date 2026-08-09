# Titanweave K14.C11 Tester Guide

Run from the extracted source root:

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c11-qemu-reviewed-registers.sh
```

Expected C11 markers:

```text
[C11RF] reviewed Radeon register definitions:
[C11BA] IP-base address resolver:
[C11HW] physical Radeon register reads:
[C11RD] K14.C11 reviewed register whitelist ready:
[C11OK] K14.C11 reviewed register/IP-base gate:
```

QEMU has no physical Radeon, so DISPLAYD must report:

```text
[displayd] K14.C11 reviewed Radeon register definitions and IP-base resolver gate online
[displayd] K14.C11 no physical Radeon in QEMU; reviewed register definitions remain safely deferred
```

Successful finish:

```text
[KERN] K14.C11 alive:
[QUAL] K14.C11 reviewed-register runtime reached intentional post-userspace halt
[HALT] BSP halted intentionally
Titanweave K14.C11 reviewed-register/runtime qualification PASSED.
```

C11 intentionally performs no real Radeon MMIO read until a trusted per-IP base map is available.
