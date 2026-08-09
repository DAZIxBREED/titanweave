# Titanweave K14.C25 Source Status

Status: **IMPLEMENTED / SOURCE-VALIDATED / RUNTIME QUALIFICATION PENDING**

K14.C25 is integrated from frozen/qualified K14.C24. It retains the exact checksum-backed GFX12 `SCRATCH_REG0` target and adds two distinct internally-derived reversible four-bit pattern/readback/restore cycles with an explicit inter-cycle restoration-persistence check. No additional Radeon register or destructive capability is enabled.
