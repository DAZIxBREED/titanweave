# K11 runtime fault diagnostics

Adds #TS/#NP/#SS IDT coverage and validates/logs the first kernel-task context frame before IRETQ. This is intended to expose the original scheduler context-switch fault instead of allowing it to collapse into an opaque #DF.
