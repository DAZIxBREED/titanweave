# Titanweave K14.C17 — AMD IP Discovery / Navi48 exact-base gate

C17 imports the public AMD IP Discovery binary layout into a bounded Rust parser. It validates the AMD binary signature `0x28211407`, IP Discovery table signature `0x53445049`, binary/table bounds, table counts, die counts, and the version-4 64-bit-base flag.

C17 does **not** read the live discovery table from Radeon VRAM yet and does not guess Navi48 IP bases. The live snapshot, checksum verification, exact GC IP entry resolution, and physical C16 promotion remain future gates. Radeon MMIO writes, firmware upload, command submission, and bus-master enable remain off.

Reference model: Linux AMDGPU `drivers/gpu/drm/amd/include/discovery.h` and `amdgpu_discovery.c`.
