# K11 runtime fix 2

This patch makes WeaveCore explicitly normalize per-CPU execution state before
its first scheduler context switch. K11 does not yet maintain CET supervisor
shadow stacks per task, so inherited CR4.CET is cleared. CR0 x87 state is
normalized and x87 is initialized. Explicit #NM (7) and #CP (21) IDT gates are
also installed so these faults cannot be hidden behind a double fault.
