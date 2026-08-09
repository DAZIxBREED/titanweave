# Titanweave K12 Frozen Baseline Status

K12 is the frozen, QEMU-qualified graphics/display/compositor baseline inherited
by K13.

Qualified K12 runtime markers proved the GOP scanout fallback, compositor
surface/damage model, input focus/capture, ForgeGraphics ABI v1, Workplace Shell
preview, DISPLAYD native service, stable userspace handoff, and intentional
post-userspace halt.

The original frozen K12 archive remains the rollback artifact. This K13 tree may
reference K12 interfaces but must not rewrite that frozen archive.
