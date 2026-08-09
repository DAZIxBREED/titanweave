# Titanweave K14.C26 Tester Guide — Final K14 Completion Gate

Run from the extracted integrated source tree on Fedora:

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c26-qemu-final-mmio-allowlist.sh
```

QEMU intentionally has no physical Radeon. Expected qualification therefore proves the explicit deferred path, full userspace/syscall integration, final MMIO allowlist policy, and intentional-HALT harness.

The serial checker requires C25 inheritance plus `[C26RV]`, `[C26AL]`, `[C26PG]`, `[C26HW]`, `[C26RD]`, `[C26OK]`, userspace C26 online/deferred markers, the historical `[K14DONE]`, `[QUAL]`, and `[HALT]`.

A successful run ends with:

```text
Titanweave K14.C26 final-k14-mmio-allowlist runtime qualification PASSED.
QEMU stopped after intentional kernel halt (raw exit status: 0)
```

C26 is qualified/frozen. The owner-locked roadmap continues K14 through C32; K15 remains ForgeAudio.
