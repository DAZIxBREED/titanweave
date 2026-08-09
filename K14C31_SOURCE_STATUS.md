# Titanweave K14.C31 Source Status

Status: **IMPLEMENTED / SOURCE-VALIDATED / RUNTIME QUALIFICATION PENDING**

Frozen prerequisite: **K14.C30 QUALIFIED / FROZEN**.

C31 implements owned shader upload, SHA-backed shader cache/precache, typed command encoding, separate compute/graphics queues, pipeline state, verified vector-add compute dispatch, verified triangle graphics draw into C30 scanout, live framebuffer presentation, timeline-fence retirement, and a stable future compute-runtime capability model.

Physical Radeon CP/GFX queue programming, native AMD ISA execution, privileged GFX MMIO and GPU page-table programming remain explicitly false. No stubs or fake-success hardware path is accepted by the C31 source gate.
Runtime fix 1 corrected the reference-shader wire-magic endianness (`TWSH` parsed with `u32::from_le_bytes`) after the first Fedora/QEMU run correctly rejected the mismatched numeric constant. The source gate now asserts the wire-format value explicitly.
