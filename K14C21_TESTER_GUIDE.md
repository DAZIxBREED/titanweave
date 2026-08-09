# Titanweave K14.C21 tester guide

## Fedora/QEMU qualification

```bash
cd ~/Downloads
rm -rf titanweave-kernel-k14c21-integrated
unzip titanweave-kernel-k14c21-integrated-source.zip
cd titanweave-kernel-k14c21-integrated

./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c21-qemu-reviewed-mmio-rebind.sh
```

The runner defaults to `K13_DISPLAY=gtk`, so the Titanweave graphical QEMU window should appear. Set `K13_DISPLAY=none` only when an intentionally headless run is desired.

QEMU has no physical Radeon. The required qualification path therefore proves source/self-test integration, runtime gating, userspace ABI, graphics fallback, and the explicit safe defer. It does **not** prove a physical Radeon MMIO store.

Expected ending:

```text
PASS  [BOOT] WeaveCore K14 entered from WEAVECORE.ELF
PASS  [C20OK] K14.C20 AMD exact live IP-base gate:
PASS  [C21RV] reviewed GFX12 target rebind:
PASS  [C21PG] post-discovery identity-write policy:
PASS  [C21HW] GFX12 SCRATCH_REG0 identity-write:
PASS  [C21RD] K14.C21 reviewed-target rebind ready:
PASS  [C21OK] K14.C21 reviewed GFX12 target rebind/identity-write gate:
PASS  [USER] [displayd] K14.C21 reviewed GFX12 SCRATCH_REG0 rebind/identity-write gate online
PASS  [USER] [displayd] K14.C21 no physical Radeon in QEMU; reviewed GFX12 identity-write remains safely deferred
PASS  [RECV] kernel initialization reached stable userspace handoff
PASS  [KERN] K14.C21 alive:
PASS  [QUAL] K14.C21 reviewed-MMIO-rebind runtime reached intentional post-userspace halt
PASS  [HALT] BSP halted intentionally
Titanweave K14.C21 reviewed-MMIO-rebind runtime qualification PASSED.
```

## Bare-metal boundary

A later explicit bare-metal qualification on a supported Navi48/RX 9070-class device is the authority for the physical C21 transaction. A QEMU PASS must never be recorded as proof that the physical MMIO store occurred.
