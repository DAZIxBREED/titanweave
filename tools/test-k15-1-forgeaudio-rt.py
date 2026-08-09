#!/usr/bin/env python3
from pathlib import Path
import re
import sys

root = Path(__file__).resolve().parents[1]
scheduler = (root / 'kernel/weavecore/src/scheduler.rs').read_text()
rt_mutex = (root / 'kernel/weavecore/src/rt_mutex.rs').read_text()
main = (root / 'kernel/weavecore/src/main.rs').read_text()
stone = (root / 'K15_STONE_CONTRACT.md').read_text()
implementation = (root / 'K15_1_IMPLEMENTATION.md').read_text()

required_scheduler = [
    'SchedulingClass::RealtimeAudio',
    'RtTaskConfig',
    'FORGEAUDIO_RT_TICK_HZ: u32 = 1_000',
    'rt_period_ticks',
    'rt_budget_ticks',
    'rt_deadline_ticks',
    'rt_absolute_deadline_tick',
    'rt_budget_remaining',
    'deadline_misses',
    'budget_exhaustions',
    'release_due_rt_jobs',
    'reserve_audio_cpu',
    'set_inherited_priority',
    'enter_preemption_guard',
    'wait_until_next_rt_period',
    'run_forgeaudio_rt_self_test',
    'RtTaskConfig::audio(24, 4, 2, 3, 5)',
    'RT_AUDIO_JOBS_COMPLETED',
    '[K15OK] K15.1 ForgeAudio real-time execution foundation qualified',
]
for token in required_scheduler:
    assert token in scheduler, token

required_mutex = [
    'pub struct RtMutex',
    'pub fn lock(&self)',
    'pub fn unlock(&self)',
    'scheduler::prepare_block_current',
    'scheduler::set_inherited_priority',
    'scheduler::clear_inherited_priority',
    'scheduler::wake_task',
    'pop_highest_waiter',
]
for token in required_mutex:
    assert token in rt_mutex, token

for token in [
    'mod rt_mutex;',
    'scheduler::run_forgeaudio_rt_self_test',
    '[K15RD] ForgeAudio RT ready:',
]:
    assert token in main, token

# Stone contract must remain exactly 16 numbered gates.
gates = re.findall(r'^\d+\. \*\*K15\.(\d+) — ', stone, flags=re.MULTILINE)
assert gates == [str(i) for i in range(1, 17)], gates
assert 'K15 ends at K15.16.' in stone

# No placeholder implementation vocabulary on the K15.1 required source path.
for path, text in [
    ('scheduler.rs', scheduler),
    ('rt_mutex.rs', rt_mutex),
]:
    lowered = text.lower()
    for forbidden in ['todo!', 'unimplemented!', 'placeholder', 'fake success', 'stub']:
        assert forbidden not in lowered, f'{path}: forbidden token {forbidden!r}'

assert 'source-integrated' in implementation.lower() or 'implemented' in implementation.lower()
print('Titanweave K15.1 ForgeAudio RT source checks passed.')
