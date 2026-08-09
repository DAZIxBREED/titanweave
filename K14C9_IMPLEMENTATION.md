# Titanweave K14.C9 — Verified Radeon Profiles + Live Safe Identity Reads

K14.C9 is forked from the frozen/qualified K14.C8 tree.

## Scope

C9 adds the first grounded Radeon device profiles used by Titanweave native bring-up and a live, side-effect-free PCI configuration identity-read path. The first targets are:

- AMD Navi 21 / Radeon RX 6800/6800 XT/6900 XT family — PCI device `1002:73bf`.
- AMD Navi 48 / Radeon RX 9070/9070 XT family — PCI device `1002:7550`, C0-family revisions.

The profiles carry only major IP-family information needed for bring-up planning. C9 intentionally does **not** populate an MMIO register whitelist. Exact MMIO offsets and semantics remain a separate promotion gate.

## Live reads permitted in C9

Only conventional PCI configuration-space identity/status reads are used: vendor/device identity, revision/class identity, and PCI command state. C9 requires the selected Radeon to agree with the live configuration-space identity and requires bus mastering to remain disabled.

## Still forbidden

- Radeon MMIO register reads
- Radeon register writes
- firmware upload
- command submission
- Radeon bus mastering

Unknown devices or revisions remain fail-closed and retain VirtIO-GPU/GOP fallback.
