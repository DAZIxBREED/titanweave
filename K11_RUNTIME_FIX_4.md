# K11 runtime fix 4: full long-mode IRET frame

The scheduler's synthetic CPL0 task frame was only 20 qwords long and stopped at RFLAGS.
In 64-bit mode IRETQ restores SS:RSP from the interrupt frame even for CPL0 -> CPL0 returns.
That caused the zero-filled words after the synthetic frame to be loaded as RSP=0 and SS=0.
The first task prologue then produced RSP=-0x88 and its first CALL page-faulted while pushing
a return address at -0x90.

This fix expands InterruptFrame to the full 22-qword long-mode hardware layout and seeds
an explicit kernel RSP and SS for newly-created kernel tasks. The scheduler now validates
and logs those fields before the first dispatch.
