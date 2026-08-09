# Titanweave K14.C31 — Graphics + Compute Execution

C31 advances from frozen K14.C30 and closes the locked graphics+compute execution milestone without pretending QEMU contains Radeon silicon.

## Operational execution path
- Driver-owned GTT shader objects with SHA-256 identities and upload readback.
- Separate bounded compute and graphics queues with queued/running/retired lifecycle.
- GTT-backed typed command buffers for bind/resource/dispatch/draw/present operations.
- Compute pipeline state and a 256-element `u32` vector-add dispatch over real C28 GTT objects.
- Numeric output verification, result fingerprinting and timeline-fence retirement.
- Graphics pipeline state with uploaded vertex/pixel reference shaders.
- Triangle rasterization into C30's real back scanout object and verified live framebuffer present.
- Bounded shader cache with lookup/hit/miss counters and precache residency.
- Stable compute-capability contract for later HIP/ROCm-style runtimes: 64-bit addressing, 3D dispatch model, separate compute/graphics queues, timeline fences, host-visible GTT, shader cache and precache.

## Authority boundary
The QEMU executor is explicitly Titanweave's reference backend. C31 does **not** claim native AMD machine-code execution, physical Radeon CP/MEC/GFX queue programming, privileged GFX MMIO, or GPU page-table programming. Those remain false unless a later/bare-metal hardware path proves the exact prerequisites. No `todo!()`, `unimplemented!()`, placeholder subsystem, or fake physical-GPU success is permitted.
