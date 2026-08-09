# K11 Runtime Fix 3 — low-memory exclusion and fatal-fault IST

- General-purpose frame allocation now excludes physical memory below 1 MiB.
- The AP trampoline remains explicitly reserved at 0x8000 by TitanBoot.
- A fourth per-CPU IST stack is installed for fatal task/context faults.
- #UD, #NM, #TS, #NP, #SS, #GP, #PF, and #CP use the fatal-fault IST so a bad task stack cannot hide the original exception behind #DF.
