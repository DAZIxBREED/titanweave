# K13.B Runtime Fix 1 — High PCI MMIO BAR Mapping

The first K13.B QEMU runtime reached the qualified K13.A foundation and then
stopped before VirtIO feature negotiation with:

`VirtIO capability lies outside bootstrap identity map`

Root cause: Q35/OVMF may assign a 64-bit PCI BAR at or above TitanBoot's
512 GiB bootstrap identity-map ceiling. PCI MMIO placement is independent of
the early direct-map policy, so requiring every BAR capability to be identity
mapped was incorrect.

Fix:
- add a 1 GiB supervisor-only kernel MMIO virtual aperture at
  `0xffff_ffc0_0000_0000`;
- map out-of-direct-map PCI capability pages on demand with 4 KiB PTEs;
- use writable, NX, cache-disabled/write-through mappings for device MMIO;
- keep the bounded 512 GiB bootstrap identity map unchanged;
- preserve ForgeBus ownership-before-DMA and K12 GOP fallback behavior;
- emit `[MMIO]` diagnostics showing physical-to-virtual capability mappings.

This is preferable to expanding the bootstrap identity map merely to follow a
QEMU BAR-placement choice and is the required groundwork for real hardware,
where GPU MMIO BARs may reside anywhere in the platform PCI aperture.
