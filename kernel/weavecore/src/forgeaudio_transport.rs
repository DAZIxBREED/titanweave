//! K15.7 ForgeAudio lock-free application/server transport.
//!
//! The required hot-path ring operations are bounded SPSC queues implemented
//! only with atomics and fixed storage. They do not allocate, take a mutex,
//! perform filesystem I/O, or sleep. Syscalls only copy a fixed block/command
//! between userspace and these queues; queue ownership and sequencing live here.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use titanweave_forgeaudio_abi::{
    AUDIO_TRANSPORT_BLOCK_BYTES, AUDIO_TRANSPORT_COMMAND_BYTES,
    AUDIO_TRANSPORT_COMMAND_DEPTH, AUDIO_TRANSPORT_MAX_SESSIONS,
    AUDIO_TRANSPORT_RING_SLOTS,
};
use crate::serial;

const SESSION_FREE: u32 = 0;
const SESSION_ATTACHED: u32 = 1;
const SESSION_DEAD: u32 = 2;
const SESSION_CLAIMING: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportError {
    NotFound,
    NotReady,
    Busy,
    Full,
    Empty,
    StaleGeneration,
    AccessDenied,
    Invalid,
}

struct BlockSlot(UnsafeCell<[u8; AUDIO_TRANSPORT_BLOCK_BYTES]>);
unsafe impl Sync for BlockSlot {}
impl BlockSlot {
    const fn new() -> Self { Self(UnsafeCell::new([0; AUDIO_TRANSPORT_BLOCK_BYTES])) }
    fn write(&self, input: &[u8; AUDIO_TRANSPORT_BLOCK_BYTES]) {
        unsafe { (*self.0.get()).copy_from_slice(input); }
    }
    fn read(&self, output: &mut [u8; AUDIO_TRANSPORT_BLOCK_BYTES]) {
        unsafe { output.copy_from_slice(&*self.0.get()); }
    }
    fn wipe(&self) { unsafe { (*self.0.get()).fill(0); } }
}

struct BlockRing {
    head: AtomicU64,
    tail: AtomicU64,
    slots: [BlockSlot; AUDIO_TRANSPORT_RING_SLOTS],
}
unsafe impl Sync for BlockRing {}
impl BlockRing {
    const fn new() -> Self {
        Self {
            head: AtomicU64::new(0),
            tail: AtomicU64::new(0),
            slots: [const { BlockSlot::new() }; AUDIO_TRANSPORT_RING_SLOTS],
        }
    }
    fn reset(&self) {
        self.head.store(0, Ordering::Release);
        self.tail.store(0, Ordering::Release);
        for slot in &self.slots { slot.wipe(); }
    }
    fn push(&self, input: &[u8; AUDIO_TRANSPORT_BLOCK_BYTES]) -> Result<u64, TransportError> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        if head.wrapping_sub(tail) >= AUDIO_TRANSPORT_RING_SLOTS as u64 {
            return Err(TransportError::Full);
        }
        let index = (head % AUDIO_TRANSPORT_RING_SLOTS as u64) as usize;
        self.slots[index].write(input);
        let next = head.wrapping_add(1);
        self.head.store(next, Ordering::Release);
        Ok(next)
    }
    fn pop(&self, output: &mut [u8; AUDIO_TRANSPORT_BLOCK_BYTES]) -> Result<u64, TransportError> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        if tail == head { return Err(TransportError::Empty); }
        let index = (tail % AUDIO_TRANSPORT_RING_SLOTS as u64) as usize;
        self.slots[index].read(output);
        let next = tail.wrapping_add(1);
        self.tail.store(next, Ordering::Release);
        Ok(next)
    }
}

struct CommandSlot(UnsafeCell<[u8; AUDIO_TRANSPORT_COMMAND_BYTES]>);
unsafe impl Sync for CommandSlot {}
impl CommandSlot {
    const fn new() -> Self { Self(UnsafeCell::new([0; AUDIO_TRANSPORT_COMMAND_BYTES])) }
    fn write(&self, input: &[u8; AUDIO_TRANSPORT_COMMAND_BYTES]) {
        unsafe { (*self.0.get()).copy_from_slice(input); }
    }
    fn read(&self, output: &mut [u8; AUDIO_TRANSPORT_COMMAND_BYTES]) {
        unsafe { output.copy_from_slice(&*self.0.get()); }
    }
    fn wipe(&self) { unsafe { (*self.0.get()).fill(0); } }
}

struct CommandRing {
    head: AtomicU64,
    tail: AtomicU64,
    slots: [CommandSlot; AUDIO_TRANSPORT_COMMAND_DEPTH],
}
unsafe impl Sync for CommandRing {}
impl CommandRing {
    const fn new() -> Self {
        Self {
            head: AtomicU64::new(0),
            tail: AtomicU64::new(0),
            slots: [const { CommandSlot::new() }; AUDIO_TRANSPORT_COMMAND_DEPTH],
        }
    }
    fn reset(&self) {
        self.head.store(0, Ordering::Release);
        self.tail.store(0, Ordering::Release);
        for slot in &self.slots { slot.wipe(); }
    }
    fn push(&self, input: &[u8; AUDIO_TRANSPORT_COMMAND_BYTES]) -> Result<u64, TransportError> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        if head.wrapping_sub(tail) >= AUDIO_TRANSPORT_COMMAND_DEPTH as u64 {
            return Err(TransportError::Full);
        }
        let index = (head % AUDIO_TRANSPORT_COMMAND_DEPTH as u64) as usize;
        self.slots[index].write(input);
        let next = head.wrapping_add(1);
        self.head.store(next, Ordering::Release);
        Ok(next)
    }
    fn pop(&self, output: &mut [u8; AUDIO_TRANSPORT_COMMAND_BYTES]) -> Result<u64, TransportError> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        if tail == head { return Err(TransportError::Empty); }
        let index = (tail % AUDIO_TRANSPORT_COMMAND_DEPTH as u64) as usize;
        self.slots[index].read(output);
        let next = tail.wrapping_add(1);
        self.tail.store(next, Ordering::Release);
        Ok(next)
    }
}

struct TransportSession {
    state: AtomicU32,
    generation: AtomicU32,
    client_pid: AtomicU64,
    server_pid: AtomicU64,
    playback: BlockRing,
    capture: BlockRing,
    client_commands: CommandRing,
    server_commands: CommandRing,
}
unsafe impl Sync for TransportSession {}
impl TransportSession {
    const fn new() -> Self {
        Self {
            state: AtomicU32::new(SESSION_FREE),
            generation: AtomicU32::new(1),
            client_pid: AtomicU64::new(0),
            server_pid: AtomicU64::new(0),
            playback: BlockRing::new(),
            capture: BlockRing::new(),
            client_commands: CommandRing::new(),
            server_commands: CommandRing::new(),
        }
    }
    fn reset_rings(&self) {
        self.playback.reset();
        self.capture.reset();
        self.client_commands.reset();
        self.server_commands.reset();
    }
}

static SESSIONS: [TransportSession; AUDIO_TRANSPORT_MAX_SESSIONS] =
    [const { TransportSession::new() }; AUDIO_TRANSPORT_MAX_SESSIONS];

// Global qualification counters persist across dead-client ring reset/reap.
static PLAYBACK_PUSHES: AtomicU64 = AtomicU64::new(0);
static PLAYBACK_POPS: AtomicU64 = AtomicU64::new(0);
static CAPTURE_PUSHES: AtomicU64 = AtomicU64::new(0);
static CAPTURE_POPS: AtomicU64 = AtomicU64::new(0);
static CLIENT_COMMAND_PUSHES: AtomicU64 = AtomicU64::new(0);
static CLIENT_COMMAND_POPS: AtomicU64 = AtomicU64::new(0);
static SERVER_COMMAND_PUSHES: AtomicU64 = AtomicU64::new(0);
static SERVER_COMMAND_POPS: AtomicU64 = AtomicU64::new(0);
static DATA_FULL_HITS: AtomicU64 = AtomicU64::new(0);
static DATA_EMPTY_HITS: AtomicU64 = AtomicU64::new(0);
static COMMAND_FULL_HITS: AtomicU64 = AtomicU64::new(0);
static COMMAND_EMPTY_HITS: AtomicU64 = AtomicU64::new(0);
static PLAYBACK_WRAPS: AtomicU64 = AtomicU64::new(0);
static CAPTURE_WRAPS: AtomicU64 = AtomicU64::new(0);
static COMMAND_WRAPS: AtomicU64 = AtomicU64::new(0);
static STALE_REJECTIONS: AtomicU64 = AtomicU64::new(0);
static DEAD_CLIENTS: AtomicU64 = AtomicU64::new(0);
static GENERATION_ADVANCES: AtomicU64 = AtomicU64::new(0);
static ATTACHES: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug)]
pub struct TransportQualification {
    pub playback_blocks: u64,
    pub capture_blocks: u64,
    pub command_roundtrips: u64,
    pub playback_wraps: u64,
    pub capture_wraps: u64,
    pub command_wraps: u64,
    pub data_full_hits: u64,
    pub data_empty_hits: u64,
    pub command_full_hits: u64,
    pub command_empty_hits: u64,
    pub stale_rejections: u64,
    pub dead_clients: u64,
    pub generation_advances: u64,
}

fn session_by_id(session_id: u32) -> Result<&'static TransportSession, TransportError> {
    if session_id == 0 || session_id as usize > SESSIONS.len() { return Err(TransportError::NotFound); }
    Ok(&SESSIONS[(session_id - 1) as usize])
}

fn check_client(session: &TransportSession, pid: u64, generation: u32) -> Result<(), TransportError> {
    if session.state.load(Ordering::Acquire) != SESSION_ATTACHED || session.client_pid.load(Ordering::Acquire) != pid {
        return Err(TransportError::AccessDenied);
    }
    if session.generation.load(Ordering::Acquire) != generation {
        STALE_REJECTIONS.fetch_add(1, Ordering::Relaxed);
        return Err(TransportError::StaleGeneration);
    }
    Ok(())
}

fn check_server(session: &TransportSession, pid: u64, generation: u32, allow_dead: bool) -> Result<(), TransportError> {
    let state = session.state.load(Ordering::Acquire);
    if session.server_pid.load(Ordering::Acquire) != pid || (state != SESSION_ATTACHED && !(allow_dead && state == SESSION_DEAD)) {
        return Err(TransportError::AccessDenied);
    }
    if session.generation.load(Ordering::Acquire) != generation {
        STALE_REJECTIONS.fetch_add(1, Ordering::Relaxed);
        return Err(TransportError::StaleGeneration);
    }
    Ok(())
}

pub fn client_attach(client_pid: u64, server_pid: u64) -> Result<(u32, u32), TransportError> {
    if client_pid == 0 || server_pid == 0 || client_pid == server_pid { return Err(TransportError::Invalid); }
    for (index, session) in SESSIONS.iter().enumerate() {
        if session.state.compare_exchange(SESSION_FREE, SESSION_CLAIMING, Ordering::AcqRel, Ordering::Acquire).is_err() {
            continue;
        }
        session.reset_rings();
        session.client_pid.store(client_pid, Ordering::Release);
        session.server_pid.store(server_pid, Ordering::Release);
        let generation = session.generation.load(Ordering::Acquire).max(1);
        session.generation.store(generation, Ordering::Release);
        session.state.store(SESSION_ATTACHED, Ordering::Release);
        ATTACHES.fetch_add(1, Ordering::Relaxed);
        serial::println(format_args!(
            "[K15LF] transport attached: session={} client_pid={} server_pid={} generation={} slots={} block_bytes={} command_depth={} lock_free=true",
            index + 1, client_pid, server_pid, generation, AUDIO_TRANSPORT_RING_SLOTS,
            AUDIO_TRANSPORT_BLOCK_BYTES, AUDIO_TRANSPORT_COMMAND_DEPTH
        ));
        return Ok(((index + 1) as u32, generation));
    }
    Err(TransportError::Busy)
}

pub fn find_active_for_server(server_pid: u64) -> Result<(u32, u32), TransportError> {
    for (index, session) in SESSIONS.iter().enumerate() {
        if session.state.load(Ordering::Acquire) == SESSION_ATTACHED && session.server_pid.load(Ordering::Acquire) == server_pid {
            return Ok(((index + 1) as u32, session.generation.load(Ordering::Acquire)));
        }
    }
    Err(TransportError::Empty)
}

pub fn find_dead_for_server(server_pid: u64) -> Result<(u32, u32), TransportError> {
    for (index, session) in SESSIONS.iter().enumerate() {
        if session.state.load(Ordering::Acquire) == SESSION_DEAD && session.server_pid.load(Ordering::Acquire) == server_pid {
            return Ok(((index + 1) as u32, session.generation.load(Ordering::Acquire)));
        }
    }
    Err(TransportError::Empty)
}

pub fn client_push_playback(session_id: u32, generation: u32, pid: u64, block: &[u8; AUDIO_TRANSPORT_BLOCK_BYTES]) -> Result<u64, TransportError> {
    let session = session_by_id(session_id)?; check_client(session, pid, generation)?;
    match session.playback.push(block) {
        Ok(sequence) => {
            PLAYBACK_PUSHES.fetch_add(1, Ordering::Relaxed);
            if sequence % AUDIO_TRANSPORT_RING_SLOTS as u64 == 0 { PLAYBACK_WRAPS.fetch_add(1, Ordering::Relaxed); }
            Ok(sequence)
        }
        Err(TransportError::Full) => { DATA_FULL_HITS.fetch_add(1, Ordering::Relaxed); Err(TransportError::Full) }
        Err(error) => Err(error),
    }
}

pub fn server_pop_playback(session_id: u32, generation: u32, pid: u64, block: &mut [u8; AUDIO_TRANSPORT_BLOCK_BYTES]) -> Result<u64, TransportError> {
    let session = session_by_id(session_id)?; check_server(session, pid, generation, false)?;
    match session.playback.pop(block) {
        Ok(sequence) => { PLAYBACK_POPS.fetch_add(1, Ordering::Relaxed); Ok(sequence) }
        Err(TransportError::Empty) => { DATA_EMPTY_HITS.fetch_add(1, Ordering::Relaxed); Err(TransportError::Empty) }
        Err(error) => Err(error),
    }
}

pub fn server_push_capture(session_id: u32, generation: u32, pid: u64, block: &[u8; AUDIO_TRANSPORT_BLOCK_BYTES]) -> Result<u64, TransportError> {
    let session = session_by_id(session_id)?; check_server(session, pid, generation, false)?;
    match session.capture.push(block) {
        Ok(sequence) => {
            CAPTURE_PUSHES.fetch_add(1, Ordering::Relaxed);
            if sequence % AUDIO_TRANSPORT_RING_SLOTS as u64 == 0 { CAPTURE_WRAPS.fetch_add(1, Ordering::Relaxed); }
            Ok(sequence)
        }
        Err(TransportError::Full) => { DATA_FULL_HITS.fetch_add(1, Ordering::Relaxed); Err(TransportError::Full) }
        Err(error) => Err(error),
    }
}

pub fn client_pop_capture(session_id: u32, generation: u32, pid: u64, block: &mut [u8; AUDIO_TRANSPORT_BLOCK_BYTES]) -> Result<u64, TransportError> {
    let session = session_by_id(session_id)?; check_client(session, pid, generation)?;
    match session.capture.pop(block) {
        Ok(sequence) => { CAPTURE_POPS.fetch_add(1, Ordering::Relaxed); Ok(sequence) }
        Err(TransportError::Empty) => { DATA_EMPTY_HITS.fetch_add(1, Ordering::Relaxed); Err(TransportError::Empty) }
        Err(error) => Err(error),
    }
}

pub fn client_push_command(session_id: u32, generation: u32, pid: u64, command: &[u8; AUDIO_TRANSPORT_COMMAND_BYTES]) -> Result<u64, TransportError> {
    let session = session_by_id(session_id)?; check_client(session, pid, generation)?;
    match session.client_commands.push(command) {
        Ok(sequence) => {
            CLIENT_COMMAND_PUSHES.fetch_add(1, Ordering::Relaxed);
            if sequence % AUDIO_TRANSPORT_COMMAND_DEPTH as u64 == 0 { COMMAND_WRAPS.fetch_add(1, Ordering::Relaxed); }
            Ok(sequence)
        }
        Err(TransportError::Full) => { COMMAND_FULL_HITS.fetch_add(1, Ordering::Relaxed); Err(TransportError::Full) }
        Err(error) => Err(error),
    }
}

pub fn server_pop_command(session_id: u32, generation: u32, pid: u64, command: &mut [u8; AUDIO_TRANSPORT_COMMAND_BYTES]) -> Result<u64, TransportError> {
    let session = session_by_id(session_id)?; check_server(session, pid, generation, false)?;
    match session.client_commands.pop(command) {
        Ok(sequence) => { CLIENT_COMMAND_POPS.fetch_add(1, Ordering::Relaxed); Ok(sequence) }
        Err(TransportError::Empty) => { COMMAND_EMPTY_HITS.fetch_add(1, Ordering::Relaxed); Err(TransportError::Empty) }
        Err(error) => Err(error),
    }
}

pub fn server_push_command(session_id: u32, generation: u32, pid: u64, command: &[u8; AUDIO_TRANSPORT_COMMAND_BYTES]) -> Result<u64, TransportError> {
    let session = session_by_id(session_id)?; check_server(session, pid, generation, false)?;
    match session.server_commands.push(command) {
        Ok(sequence) => {
            SERVER_COMMAND_PUSHES.fetch_add(1, Ordering::Relaxed);
            if sequence % AUDIO_TRANSPORT_COMMAND_DEPTH as u64 == 0 { COMMAND_WRAPS.fetch_add(1, Ordering::Relaxed); }
            Ok(sequence)
        }
        Err(TransportError::Full) => { COMMAND_FULL_HITS.fetch_add(1, Ordering::Relaxed); Err(TransportError::Full) }
        Err(error) => Err(error),
    }
}

pub fn client_pop_command(session_id: u32, generation: u32, pid: u64, command: &mut [u8; AUDIO_TRANSPORT_COMMAND_BYTES]) -> Result<u64, TransportError> {
    let session = session_by_id(session_id)?; check_client(session, pid, generation)?;
    match session.server_commands.pop(command) {
        Ok(sequence) => { SERVER_COMMAND_POPS.fetch_add(1, Ordering::Relaxed); Ok(sequence) }
        Err(TransportError::Empty) => { COMMAND_EMPTY_HITS.fetch_add(1, Ordering::Relaxed); Err(TransportError::Empty) }
        Err(error) => Err(error),
    }
}

pub fn detach_process(pid: u64) {
    if pid == 0 { return; }
    for (index, session) in SESSIONS.iter().enumerate() {
        if session.state.load(Ordering::Acquire) != SESSION_ATTACHED || session.client_pid.load(Ordering::Acquire) != pid {
            continue;
        }
        if session.state.compare_exchange(SESSION_ATTACHED, SESSION_DEAD, Ordering::AcqRel, Ordering::Acquire).is_err() {
            continue;
        }
        let old_generation = session.generation.fetch_add(1, Ordering::AcqRel);
        let new_generation = old_generation.wrapping_add(1).max(1);
        session.client_pid.store(0, Ordering::Release);
        session.reset_rings();
        DEAD_CLIENTS.fetch_add(1, Ordering::Relaxed);
        GENERATION_ADVANCES.fetch_add(1, Ordering::Relaxed);
        serial::println(format_args!(
            "[K15LF] dead client isolated: session={} old_generation={} new_generation={} rings_reset=true server_alive=true",
            index + 1, old_generation, new_generation
        ));
    }
}

pub fn reap_dead(session_id: u32, generation: u32, server_pid: u64) -> Result<(), TransportError> {
    let session = session_by_id(session_id)?;
    check_server(session, server_pid, generation, true)?;
    if session.state.load(Ordering::Acquire) != SESSION_DEAD { return Err(TransportError::Invalid); }
    session.server_pid.store(0, Ordering::Release);
    session.state.store(SESSION_FREE, Ordering::Release);
    Ok(())
}

pub fn force_stale_probe(session_id: u32, stale_generation: u32, server_pid: u64) -> Result<(), TransportError> {
    let session = session_by_id(session_id)?;
    check_server(session, server_pid, stale_generation, true)
}

pub fn qualification_snapshot() -> Result<TransportQualification, TransportError> {
    let playback_pushes = PLAYBACK_PUSHES.load(Ordering::Acquire);
    let playback_pops = PLAYBACK_POPS.load(Ordering::Acquire);
    let capture_pushes = CAPTURE_PUSHES.load(Ordering::Acquire);
    let capture_pops = CAPTURE_POPS.load(Ordering::Acquire);
    let client_command_pushes = CLIENT_COMMAND_PUSHES.load(Ordering::Acquire);
    let client_command_pops = CLIENT_COMMAND_POPS.load(Ordering::Acquire);
    let server_command_pushes = SERVER_COMMAND_PUSHES.load(Ordering::Acquire);
    let server_command_pops = SERVER_COMMAND_POPS.load(Ordering::Acquire);
    let snapshot = TransportQualification {
        playback_blocks: playback_pushes.min(playback_pops),
        capture_blocks: capture_pushes.min(capture_pops),
        command_roundtrips: client_command_pushes.min(client_command_pops).min(server_command_pushes).min(server_command_pops),
        playback_wraps: PLAYBACK_WRAPS.load(Ordering::Acquire),
        capture_wraps: CAPTURE_WRAPS.load(Ordering::Acquire),
        command_wraps: COMMAND_WRAPS.load(Ordering::Acquire),
        data_full_hits: DATA_FULL_HITS.load(Ordering::Acquire),
        data_empty_hits: DATA_EMPTY_HITS.load(Ordering::Acquire),
        command_full_hits: COMMAND_FULL_HITS.load(Ordering::Acquire),
        command_empty_hits: COMMAND_EMPTY_HITS.load(Ordering::Acquire),
        stale_rejections: STALE_REJECTIONS.load(Ordering::Acquire),
        dead_clients: DEAD_CLIENTS.load(Ordering::Acquire),
        generation_advances: GENERATION_ADVANCES.load(Ordering::Acquire),
    };
    if ATTACHES.load(Ordering::Acquire) != 1
        || snapshot.playback_blocks != 12
        || snapshot.capture_blocks != 12
        || snapshot.command_roundtrips != 16
        || snapshot.playback_wraps != 3
        || snapshot.capture_wraps != 3
        || snapshot.command_wraps != 2
        || snapshot.data_full_hits < 6
        || snapshot.data_empty_hits < 6
        || snapshot.command_full_hits < 2
        || snapshot.command_empty_hits < 2
        || snapshot.stale_rejections < 1
        || snapshot.dead_clients != 1
        || snapshot.generation_advances != 1
    {
        return Err(TransportError::NotReady);
    }
    Ok(snapshot)
}
