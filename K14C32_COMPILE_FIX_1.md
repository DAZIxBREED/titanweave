# Titanweave K14.C32 Compile Fix 1

Status: **APPLIED / RUNTIME REQUALIFICATION REQUIRED**

Fedora compilation exposed one C32 namespace error in `radeon_stability.rs`:
`display::MAX_DISPLAY_CONNECTORS` referenced an undeclared module name even though the
module was imported as `radeon_display`.

The production/stability multi-display capacity check now uses:

`radeon_display::MAX_DISPLAY_CONNECTORS`

The C32 source regression also asserts the correct imported module name and rejects the
stale unresolved alias. No frozen C31 behavior was changed.
