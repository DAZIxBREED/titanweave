# Titanweave K14.C32 Tester Guide

## Fedora/QEMU final K14 qualification

From the extracted K14.C32 directory:

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c32-qemu-production-stability-final.sh
```

The halt-aware runner automatically terminates QEMU after `[HALT] BSP halted intentionally` and then checks all C32 markers.

Expected final lines include:

- `[C32QS] queue stability:`
- `[C32MP] memory pressure:`
- `[C32RC] recovery+interrupt stress:`
- `[C32CX] concurrency:`
- `[C32MD] display stability:`
- `[C32PM] power policy:`
- `[C32TL] telemetry/diagnostics:`
- `[C32AB] frozen GPU ABI/capabilities:`
- `[C32PG] final authority:`
- `[C32RD] K14.C32 production/stability final ready:`
- `[C32OK] K14.C32 production/stability + final K14:`
- both C32 `displayd` messages
- `[RECV] kernel initialization reached stable userspace handoff`
- `[KERN] K14.C32 alive:`
- `[QUAL] K14.C32 production-stability-final runtime reached intentional post-userspace halt`
- `[K14DONE] Titanweave native Radeon driver foundation operational`
- `[K15NEXT] K15 ForgeAudio is the next locked Titanweave milestone`
- `[HALT] BSP halted intentionally`

QEMU is also required to say `physical_stress_qualified=false` in `[C32PG]`. That is a correctness condition, not a failure.

## Physical Radeon evidence

For a future serial log captured on real Radeon hardware, run:

```bash
./tools/check-k14c32-baremetal-log.sh /path/to/serial.log
```

That checker requires `amd_present=true` and `physical_stress=true` and rejects logs that explicitly say physical stress is unqualified. Current QEMU qualification cannot satisfy it by design.
