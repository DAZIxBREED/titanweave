#!/usr/bin/env python3
"""Behavioral regression tests for Titanweave K1-K9 policy contracts.
These tests do not replace Rust compilation or QEMU/hardware boot tests.
"""
from dataclasses import dataclass

MAX_ARCHIVE_FILES=1_000_000
MAX_EXPANSION_RATIO=10_000


def validate_path(p: bytes):
    if not p: raise ValueError('empty')
    if b'\x00' in p: raise ValueError('nul')
    if p[:1] in (b'/', b'\\'): raise ValueError('absolute')
    if len(p)>=2 and p[1:2]==b':': raise ValueError('drive')
    parts=p.replace(b'\\',b'/').split(b'/')
    if any(x in (b'',b'.',b'..') for x in parts): raise ValueError('component')
    return True


def validate_expansion(c,e,f):
    if f>MAX_ARCHIVE_FILES: raise ValueError('files')
    if c==0 and e: raise ValueError('zero')
    if c and e>c*MAX_EXPANSION_RATIO: raise ValueError('ratio')
    return True

for good in [b'a.txt',b'dir/a.bin',b'dir\\a.bin']:
    assert validate_path(good)
for bad in [b'',b'/etc/passwd',b'\\server\\x',b'C:evil',b'a/../b',b'a//b',b'a/./b',b'a\x00b']:
    try: validate_path(bad)
    except ValueError: pass
    else: raise AssertionError(bad)
assert validate_expansion(10,100_000,12)
for args in [(0,1,1),(1,10_001,1),(1,1,1_000_001)]:
    try: validate_expansion(*args)
    except ValueError: pass
    else: raise AssertionError(args)

CREATED, VERIFIED, STAGED, ROLLBACK_READY, COMMITTING, COMMITTED, ROLLING_BACK, ROLLED_BACK, FAILED = range(9)
@dataclass
class Tx:
    state:int=CREATED
    staged:int=0
    applied:int=0
    seq:int=1
    def adv(self,s): self.seq+=1; self.state=s
    def verify(self,sig=True,sums=True):
        if self.state!=CREATED: raise ValueError('state')
        if not(sig and sums): self.adv(FAILED); raise ValueError('verify')
        self.adv(VERIFIED)
    def stage(self,n):
        if self.state!=VERIFIED or n<=0: raise ValueError('stage')
        self.staged=n; self.applied=0; self.adv(STAGED)
    def prep(self):
        if self.state!=STAGED: raise ValueError('prep')
        self.adv(ROLLBACK_READY)
    def begin_commit(self):
        if self.state!=ROLLBACK_READY: raise ValueError('commit')
        self.adv(COMMITTING)
    def apply(self):
        if self.state!=COMMITTING or self.applied>=self.staged: raise ValueError('apply')
        self.applied+=1; self.seq+=1
    def finish(self):
        if self.state!=COMMITTING or self.applied!=self.staged: raise ValueError('finish')
        self.adv(COMMITTED)
    def rollback(self):
        if self.state not in (ROLLBACK_READY,COMMITTING,FAILED): raise ValueError('rollback')
        self.adv(ROLLING_BACK); self.applied=0; self.adv(ROLLED_BACK)

t=Tx(); t.verify(); t.stage(3); t.prep(); t.begin_commit(); t.apply()
try: t.finish()
except ValueError: pass
else: raise AssertionError('partial commit accepted')
t.rollback(); assert t.state==ROLLED_BACK and t.applied==0

t=Tx(); t.verify(); t.stage(2); t.prep(); t.begin_commit(); t.apply(); t.apply(); t.finish(); assert t.state==COMMITTED

QUEUED,RUNNING,COMPLETED,JOB_FAILED,CANCELLED=range(5)
@dataclass
class Job:
    state:int=QUEUED
    owner:int=0
    result:int=0
    def claim(self,pid):
        if self.state!=QUEUED or pid==0: raise ValueError('claim')
        self.state=RUNNING; self.owner=pid
    def complete(self,pid):
        if self.state!=RUNNING or self.owner!=pid: raise ValueError('owner')
        self.state=COMPLETED
    def fail(self,pid,code):
        if self.state!=RUNNING or self.owner!=pid or code==0: raise ValueError('fail')
        self.state=JOB_FAILED; self.result=code

j=Job(); j.claim(7)
try: j.complete(8)
except ValueError: pass
else: raise AssertionError('foreign worker completed job')
j.complete(7); assert j.state==COMPLETED

print('Titanweave K1-K9 policy and transaction regression tests passed.')
