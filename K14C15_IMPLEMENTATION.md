# K14.C15 — First Controlled Radeon Write Transaction

K14.C15 advances frozen K14.C14 by qualifying the first real write-side operation without touching Radeon MMIO.

## Transaction

When a physical Radeon is present and every C14 prerequisite is complete, Titanweave reads the 16-bit PCI Command word, requires the bus-master bit to be clear, writes the exact same 16-bit value back, then reads it again. The transaction is qualified only if the readback is identical and bus mastering remains disabled.

A dedicated `pci::write_u16()` helper performs a width-correct 16-bit CONFIG_DATA write so the adjacent PCI Status word is not rewritten.

If the first readback differs, C15 performs one bounded rollback write of the original value and verifies it. A transaction that needs rollback is treated as a qualification failure even when restoration succeeds.

## Still fenced

- Radeon MMIO writes: disabled
- firmware upload: disabled
- command/ring submission: disabled
- Radeon bus-master enable: disabled

C15 proves write serialization/readback/rollback plumbing. It does not claim GPU initialization. The first Radeon MMIO write requires a separately reviewed, generation-specific register target.
