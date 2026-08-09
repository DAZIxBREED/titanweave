# Titanweave K14.C30 Runtime Qualification

Status: **QUALIFIED / FROZEN**

Fedora/QEMU runtime qualification passed on 2026-08-09. The run passed `[BOOT]`, frozen `[C29OK]`, all C30 gates (`[C30ED]`, `[C30CN]`, `[C30SC]`, `[C30MS]`, `[C30HP]`, `[C30DC]`, `[C30PG]`, `[C30RD]`, `[C30OK]`), the C30 displayd online/QEMU-GOP messages, stable userspace handoff, `[K14FOUND]`, `[KERN]`, `[QUAL]`, and intentional `[HALT]`. The halt-aware harness terminated QEMU with raw exit status 0.

QEMU qualified the real GOP-backed EDID/mode, connector/CRTC/plane, double-buffered GTT scanout, live framebuffer page-flip, atomic rollback, and hotplug-bookkeeping path. QEMU has no physical Radeon, so native DCN MMIO programming and physical HPD remain a separate bare-metal proof and are not claimed by C30.
