# Titanweave K14.C19 — bounded physical AMD discovery snapshot acquisition

K14.C19 opens the first physical AMD IP-discovery snapshot read path while keeping every GPU write-side capability fenced.

## Source-backed acquisition contract

C19 follows the same high-level path used by upstream AMDGPU for discovery data, but deliberately implements only the pure CPU read-only subset:

1. Read `DRIVER_SCRATCH_0/1/2` from Radeon register BAR5 to obtain the discovery TMR offset and size when firmware publishes them.
2. If the scratch size is zero, use the upstream VRAM-tail default (`10 KiB` at `VRAM size - 64 KiB`) when `RCC_CONFIG_MEMSIZE` is valid.
3. Discover the selected device's PCIe Resizable BAR extended capability through ACPI MCFG/ECAM.
4. Determine the *current* BAR0 aperture size without writing PCI configuration space.
5. Permit a read only when the entire TMR range fits inside the already-visible BAR0 aperture.
6. Map only that TMR range supervisor-only, NX, uncached, and read-only.
7. Copy the bytes with volatile CPU reads and pass the snapshot to K14.C18's binary + IP table checksum verifier.
8. Recheck PCI bus mastering remains OFF.

## Why MM_INDEX is still disabled

Upstream `amdgpu_device_vram_access()` first uses the CPU-visible VRAM aperture and then falls back to an MM-indexed access path for the remainder. The MM-indexed path requires register writes to select a VRAM address. C19 does not promote those writes. If BAR0 does not cover the discovery TMR, C19 safely defers instead of emulating the fallback.

## Safety boundary

C19 never resizes BAR0, never enables bus mastering, never writes Radeon MMIO, never uploads firmware, and never submits GPU commands. A snapshot is not marked verified unless C18 validates both checksums. C19 intentionally leaves exact GC/SDMA base extraction for the next milestone.
