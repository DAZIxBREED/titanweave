# Titanweave K14.C32 Runtime Fix 2

Status: **SOURCE FIXED / RUNTIME REQUALIFICATION REQUIRED**

The first C32 QEMU runtime reached every kernel production/stability gate, userspace handoff, `[QUAL]`, `[K14DONE]`, `[K15NEXT]`, and intentional `[HALT]`, but the serial qualification checker failed the second DISPLAYD marker.

Root cause: the C32 QEMU DISPLAYD qualification banner was 321 bytes while `SYS_WRITE` is limited by `MAX_MESSAGE_BYTES = 256`. The kernel correctly rejected that userspace write.

Fix:
- shorten the C32 QEMU DISPLAYD qualification banner to 243 bytes, preserving the same qualification meaning;
- update the serial checker to match the emitted banner exactly;
- add a source regression assertion that the banner is at most 256 bytes and that checker/emitter strings match.

No C31 or earlier frozen behavior changed. C32 requires one more QEMU qualification run after this source fix.
