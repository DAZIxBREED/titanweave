# Titanweave K14.C8 tester guide

Run:

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c8-qemu-asic-ip.sh
```

Expected C8 markers:

- `[C8ID] Radeon ASIC/IP identity gate:`
- `[C8RR] safe register-read policy:`
- `[C8IP] IP manifest policy:`
- `[C8HW] physical Radeon identification:`
- `[C8RD] K14.C8 Radeon ASIC/IP identification ready:`
- `[C8NF] K14.C8 Radeon ASIC/IP identification ready:`

In QEMU the physical Radeon path must remain deferred. Successful qualification ends with:

`Titanweave K14.C8 Radeon ASIC-IP/runtime qualification PASSED.`
