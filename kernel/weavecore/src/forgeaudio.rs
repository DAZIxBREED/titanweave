//! K15.2 ForgeAudio kernel object manager and lifecycle.
//!
//! Hardware drivers register real devices/endpoints here. Until a backend is
//! present the device registry remains empty; the ABI never invents a device.
//! Buffers, clocks, events and fences are real bounded kernel objects and are
//! already usable by later K15 gates.

use crate::scheduler;
use crate::serial;
use crate::sync::SpinLock;
use titanweave_forgeaudio_abi::{
    AudioAbiInfo, AudioBufferInfo, AudioClockSnapshot, AudioDeviceInfo, AudioDirection,
    AudioEndpointInfo, AudioEventKind, AudioEventRecord, AudioFenceInfo, AudioObjectKind,
    AudioSampleFormat, AudioStreamConfig, AudioStreamState, AUDIO_BUFFER_FLAG_READABLE,
    AUDIO_BUFFER_FLAG_WRITABLE, AUDIO_CLOCK_FLAG_MONOTONIC, AUDIO_FENCE_FLAG_SIGNALED,
    FORGEAUDIO_ABI_MIN_COMPATIBLE_VERSION, FORGEAUDIO_ABI_VERSION, FORGEAUDIO_FEATURES_V1,
};

pub const MAX_AUDIO_DEVICES: usize = 16;
pub const MAX_AUDIO_ENDPOINTS: usize = 64;
pub const MAX_AUDIO_STREAMS: usize = 64;
pub const MAX_AUDIO_BUFFERS: usize = 32;
pub const MAX_AUDIO_CLOCKS: usize = 32;
pub const MAX_AUDIO_EVENTS: usize = 32;
pub const MAX_AUDIO_FENCES: usize = 64;
pub const MAX_ABI_BUFFER_BYTES: usize = 16 * 1024;
pub const EVENT_QUEUE_DEPTH: usize = 16;

const OBJECT_ID_BASE: u64 = 0xA150_0000_0000_0001;
const EMPTY_DEVICE_INFO: AudioDeviceInfo = AudioDeviceInfo {
    object_id: 0,
    generation: 0,
    flags: 0,
    vendor_id: 0,
    device_id: 0,
    subsystem_vendor_id: 0,
    subsystem_device_id: 0,
    bus: 0,
    device: 0,
    function: 0,
    backend_kind: 0,
    endpoint_count: 0,
    clock_domain: 0,
    name: [0; 32],
    reserved2: 0,
};
const EMPTY_ENDPOINT_INFO: AudioEndpointInfo = AudioEndpointInfo {
    object_id: 0,
    device_object_id: 0,
    generation: 0,
    direction: 0,
    flags: 0,
    min_channels: 0,
    max_channels: 0,
    min_rate_hz: 0,
    max_rate_hz: 0,
    format_mask: 0,
    reserved: 0,
    name: [0; 32],
};
const EMPTY_STREAM_CONFIG: AudioStreamConfig = AudioStreamConfig {
    abi_version: FORGEAUDIO_ABI_VERSION,
    flags: 0,
    direction: AudioDirection::Playback as u32,
    sample_format: AudioSampleFormat::S16 as u32,
    sample_rate_hz: 48_000,
    channels: 2,
    reserved0: 0,
    period_frames: 128,
    buffer_frames: 512,
    reserved1: 0,
};
const EMPTY_EVENT: AudioEventRecord = AudioEventRecord {
    sequence: 0,
    object_id: 0,
    timestamp_tick: 0,
    kind: 0,
    code: 0,
    value0: 0,
    value1: 0,
};

#[derive(Clone, Copy)]
struct DeviceSlot {
    occupied: bool,
    generation: u32,
    open_references: u32,
    info: AudioDeviceInfo,
}
impl DeviceSlot {
    const EMPTY: Self = Self {
        occupied: false,
        generation: 0,
        open_references: 0,
        info: EMPTY_DEVICE_INFO,
    };
}

#[derive(Clone, Copy)]
struct EndpointSlot {
    occupied: bool,
    generation: u32,
    info: AudioEndpointInfo,
}
impl EndpointSlot {
    const EMPTY: Self = Self {
        occupied: false,
        generation: 0,
        info: EMPTY_ENDPOINT_INFO,
    };
}

#[derive(Clone, Copy)]
struct StreamSlot {
    occupied: bool,
    object_id: u64,
    generation: u32,
    device_object_id: u64,
    endpoint_object_id: u64,
    owner_pid: u64,
    state: AudioStreamState,
    configured: bool,
    config: AudioStreamConfig,
    frame_position: u64,
}
impl StreamSlot {
    const EMPTY: Self = Self {
        occupied: false,
        object_id: 0,
        generation: 0,
        device_object_id: 0,
        endpoint_object_id: 0,
        owner_pid: 0,
        state: AudioStreamState::Closed,
        configured: false,
        config: EMPTY_STREAM_CONFIG,
        frame_position: 0,
    };
}

#[derive(Clone, Copy)]
struct BufferSlot {
    occupied: bool,
    object_id: u64,
    generation: u32,
    owner_pid: u64,
    flags: u32,
    byte_capacity: u32,
    frame_stride_bytes: u32,
    committed_bytes: u32,
    sequence: u64,
    bytes: [u8; MAX_ABI_BUFFER_BYTES],
}
impl BufferSlot {
    const EMPTY: Self = Self {
        occupied: false,
        object_id: 0,
        generation: 0,
        owner_pid: 0,
        flags: 0,
        byte_capacity: 0,
        frame_stride_bytes: 0,
        committed_bytes: 0,
        sequence: 0,
        bytes: [0; MAX_ABI_BUFFER_BYTES],
    };
}

#[derive(Clone, Copy)]
struct ClockSlot {
    occupied: bool,
    object_id: u64,
    generation: u32,
    owner_pid: u64,
    base_tick: u64,
    rate_numerator: u32,
    rate_denominator: u32,
}
impl ClockSlot {
    const EMPTY: Self = Self {
        occupied: false,
        object_id: 0,
        generation: 0,
        owner_pid: 0,
        base_tick: 0,
        rate_numerator: 0,
        rate_denominator: 0,
    };
}

#[derive(Clone, Copy)]
struct EventSlot {
    occupied: bool,
    object_id: u64,
    generation: u32,
    owner_pid: u64,
    records: [AudioEventRecord; EVENT_QUEUE_DEPTH],
    head: usize,
    count: usize,
    next_sequence: u64,
}
impl EventSlot {
    const EMPTY: Self = Self {
        occupied: false,
        object_id: 0,
        generation: 0,
        owner_pid: 0,
        records: [EMPTY_EVENT; EVENT_QUEUE_DEPTH],
        head: 0,
        count: 0,
        next_sequence: 1,
    };
}

#[derive(Clone, Copy)]
struct FenceSlot {
    occupied: bool,
    object_id: u64,
    generation: u32,
    owner_pid: u64,
    target_value: u64,
    completed_value: u64,
    sequence: u64,
}
impl FenceSlot {
    const EMPTY: Self = Self {
        occupied: false,
        object_id: 0,
        generation: 0,
        owner_pid: 0,
        target_value: 0,
        completed_value: 0,
        sequence: 0,
    };
}

struct ForgeAudioState {
    initialized: bool,
    next_object_id: u64,
    devices: [DeviceSlot; MAX_AUDIO_DEVICES],
    endpoints: [EndpointSlot; MAX_AUDIO_ENDPOINTS],
    streams: [StreamSlot; MAX_AUDIO_STREAMS],
    buffers: [BufferSlot; MAX_AUDIO_BUFFERS],
    clocks: [ClockSlot; MAX_AUDIO_CLOCKS],
    events: [EventSlot; MAX_AUDIO_EVENTS],
    fences: [FenceSlot; MAX_AUDIO_FENCES],
}

impl ForgeAudioState {
    const fn new() -> Self {
        Self {
            initialized: false,
            next_object_id: OBJECT_ID_BASE,
            devices: [DeviceSlot::EMPTY; MAX_AUDIO_DEVICES],
            endpoints: [EndpointSlot::EMPTY; MAX_AUDIO_ENDPOINTS],
            streams: [StreamSlot::EMPTY; MAX_AUDIO_STREAMS],
            buffers: [BufferSlot::EMPTY; MAX_AUDIO_BUFFERS],
            clocks: [ClockSlot::EMPTY; MAX_AUDIO_CLOCKS],
            events: [EventSlot::EMPTY; MAX_AUDIO_EVENTS],
            fences: [FenceSlot::EMPTY; MAX_AUDIO_FENCES],
        }
    }

    fn allocate_object_id(&mut self) -> Result<u64, &'static str> {
        let id = self.next_object_id;
        self.next_object_id = self
            .next_object_id
            .checked_add(1)
            .ok_or("ForgeAudio object id space exhausted")?;
        Ok(id)
    }
}

static STATE: SpinLock<ForgeAudioState> = SpinLock::new(ForgeAudioState::new());

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioObjectRef {
    pub kind: AudioObjectKind,
    pub object_id: u64,
    pub generation: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamPosition {
    pub state: AudioStreamState,
    pub frame_position: u64,
}

#[derive(Clone, Copy)]
struct StreamStateMachine {
    state: AudioStreamState,
    configured: bool,
}

impl StreamStateMachine {
    const fn new() -> Self {
        Self {
            state: AudioStreamState::Created,
            configured: false,
        }
    }

    fn configure(&mut self) -> Result<(), &'static str> {
        if !matches!(
            self.state,
            AudioStreamState::Created | AudioStreamState::Configured | AudioStreamState::Stopped
        ) {
            return Err("stream configuration is invalid in current state");
        }
        self.configured = true;
        self.state = AudioStreamState::Configured;
        Ok(())
    }

    fn prepare(&mut self) -> Result<(), &'static str> {
        if !self.configured || !matches!(self.state, AudioStreamState::Configured | AudioStreamState::Stopped) {
            return Err("stream must be configured before prepare");
        }
        self.state = AudioStreamState::Prepared;
        Ok(())
    }

    fn start(&mut self) -> Result<(), &'static str> {
        if self.state != AudioStreamState::Prepared {
            return Err("stream must be prepared before start");
        }
        self.state = AudioStreamState::Running;
        Ok(())
    }

    fn drain(&mut self) -> Result<(), &'static str> {
        if self.state != AudioStreamState::Running {
            return Err("only a running stream can drain");
        }
        self.state = AudioStreamState::Draining;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), &'static str> {
        if !matches!(
            self.state,
            AudioStreamState::Running | AudioStreamState::Draining | AudioStreamState::Prepared
        ) {
            return Err("stream cannot stop from current state");
        }
        self.state = AudioStreamState::Stopped;
        Ok(())
    }

    fn fault(&mut self) {
        self.state = AudioStreamState::Faulted;
    }

    fn recover(&mut self) -> Result<(), &'static str> {
        if self.state != AudioStreamState::Faulted || !self.configured {
            return Err("only a configured faulted stream can recover");
        }
        self.state = AudioStreamState::Configured;
        Ok(())
    }
}

pub fn initialize() -> Result<AudioAbiInfo, &'static str> {
    let mut state = STATE.lock();
    if state.initialized {
        return Err("ForgeAudio kernel ABI already initialized");
    }
    state.initialized = true;
    drop(state);
    let info = abi_info();
    serial::println(format_args!(
        "[K15ABI] ForgeAudio ABI v{} online: devices={} endpoints={} streams={} buffers={} clocks={} events={} fences={}",
        info.current_version,
        info.max_devices,
        info.max_endpoints,
        info.max_streams,
        info.max_buffers,
        info.max_clocks,
        info.max_events,
        info.max_fences
    ));
    Ok(info)
}

#[must_use]
pub const fn abi_info() -> AudioAbiInfo {
    AudioAbiInfo {
        current_version: FORGEAUDIO_ABI_VERSION,
        minimum_compatible_version: FORGEAUDIO_ABI_MIN_COMPATIBLE_VERSION,
        features: FORGEAUDIO_FEATURES_V1,
        reserved: 0,
        max_devices: MAX_AUDIO_DEVICES as u32,
        max_endpoints: MAX_AUDIO_ENDPOINTS as u32,
        max_streams: MAX_AUDIO_STREAMS as u32,
        max_buffers: MAX_AUDIO_BUFFERS as u32,
        max_clocks: MAX_AUDIO_CLOCKS as u32,
        max_events: MAX_AUDIO_EVENTS as u32,
        max_fences: MAX_AUDIO_FENCES as u32,
        reserved2: 0,
    }
}

pub fn register_device(mut info: AudioDeviceInfo) -> Result<AudioDeviceInfo, &'static str> {
    if info.vendor_id == 0 || info.device_id == 0 || info.backend_kind == 0 {
        return Err("real audio device registration requires PCI identity and backend kind");
    }
    if info.name[0] == 0 {
        return Err("audio device registration requires a non-empty name");
    }
    let mut state = STATE.lock();
    if !state.initialized {
        return Err("ForgeAudio ABI is not initialized");
    }
    if state.devices.iter().any(|slot| {
        slot.occupied
            && slot.info.bus == info.bus
            && slot.info.device == info.device
            && slot.info.function == info.function
    }) {
        return Err("audio device PCI function already registered");
    }
    let index = state
        .devices
        .iter()
        .position(|slot| !slot.occupied)
        .ok_or("ForgeAudio device table is full")?;
    let object_id = state.allocate_object_id()?;
    let generation = state.devices[index].generation.wrapping_add(1).max(1);
    info.object_id = object_id;
    info.generation = generation;
    info.endpoint_count = 0;
    state.devices[index] = DeviceSlot {
        occupied: true,
        generation,
        open_references: 0,
        info,
    };
    Ok(info)
}

pub fn register_endpoint(
    device_object_id: u64,
    mut info: AudioEndpointInfo,
) -> Result<AudioEndpointInfo, &'static str> {
    if AudioDirection::from_raw(info.direction).is_none() {
        return Err("audio endpoint has invalid direction");
    }
    if info.min_channels == 0 || info.max_channels < info.min_channels {
        return Err("audio endpoint channel range is invalid");
    }
    if info.min_rate_hz < 8_000 || info.max_rate_hz < info.min_rate_hz || info.format_mask == 0 {
        return Err("audio endpoint format/rate range is invalid");
    }
    if info.name[0] == 0 {
        return Err("audio endpoint registration requires a non-empty name");
    }
    let mut state = STATE.lock();
    let device_index = state
        .devices
        .iter()
        .position(|slot| slot.occupied && slot.info.object_id == device_object_id)
        .ok_or("parent audio device is not registered")?;
    let endpoint_index = state
        .endpoints
        .iter()
        .position(|slot| !slot.occupied)
        .ok_or("ForgeAudio endpoint table is full")?;
    let object_id = state.allocate_object_id()?;
    let generation = state.endpoints[endpoint_index].generation.wrapping_add(1).max(1);
    info.object_id = object_id;
    info.device_object_id = device_object_id;
    info.generation = generation;
    state.endpoints[endpoint_index] = EndpointSlot {
        occupied: true,
        generation,
        info,
    };
    state.devices[device_index].info.endpoint_count = state.devices[device_index]
        .info
        .endpoint_count
        .checked_add(1)
        .ok_or("audio endpoint count overflow")?;
    Ok(info)
}

#[must_use]
pub fn device_count() -> usize {
    let state = STATE.lock();
    state.devices.iter().filter(|slot| slot.occupied).count()
}

#[must_use]
pub fn enumerate_device(index: usize) -> Option<AudioDeviceInfo> {
    let state = STATE.lock();
    let mut current = 0usize;
    for slot in &state.devices {
        if slot.occupied {
            if current == index {
                return Some(slot.info);
            }
            current += 1;
        }
    }
    None
}

#[must_use]
pub fn enumerate_endpoint(device_object_id: u64, index: usize) -> Option<AudioEndpointInfo> {
    let state = STATE.lock();
    let mut current = 0usize;
    for slot in &state.endpoints {
        if slot.occupied && slot.info.device_object_id == device_object_id {
            if current == index {
                return Some(slot.info);
            }
            current += 1;
        }
    }
    None
}

pub fn open_device(object_id: u64) -> Result<AudioObjectRef, &'static str> {
    let mut state = STATE.lock();
    let slot = state
        .devices
        .iter_mut()
        .find(|slot| slot.occupied && slot.info.object_id == object_id)
        .ok_or("audio device not found")?;
    slot.open_references = slot
        .open_references
        .checked_add(1)
        .ok_or("audio device reference overflow")?;
    Ok(AudioObjectRef {
        kind: AudioObjectKind::Device,
        object_id,
        generation: slot.generation,
    })
}

pub fn open_stream(
    owner_pid: u64,
    device_object_id: u64,
    endpoint_object_id: u64,
) -> Result<AudioObjectRef, &'static str> {
    let mut state = STATE.lock();
    if !state
        .devices
        .iter()
        .any(|slot| slot.occupied && slot.info.object_id == device_object_id)
    {
        return Err("audio stream parent device not found");
    }
    if !state.endpoints.iter().any(|slot| {
        slot.occupied
            && slot.info.object_id == endpoint_object_id
            && slot.info.device_object_id == device_object_id
    }) {
        return Err("audio stream endpoint does not belong to device");
    }
    let index = state
        .streams
        .iter()
        .position(|slot| !slot.occupied)
        .ok_or("ForgeAudio stream table is full")?;
    let object_id = state.allocate_object_id()?;
    let generation = state.streams[index].generation.wrapping_add(1).max(1);
    state.streams[index] = StreamSlot {
        occupied: true,
        object_id,
        generation,
        device_object_id,
        endpoint_object_id,
        owner_pid,
        state: AudioStreamState::Created,
        configured: false,
        config: EMPTY_STREAM_CONFIG,
        frame_position: 0,
    };
    Ok(AudioObjectRef {
        kind: AudioObjectKind::Stream,
        object_id,
        generation,
    })
}

pub fn configure_stream(object_id: u64, config: AudioStreamConfig) -> Result<(), &'static str> {
    config.validate()?;
    let mut state = STATE.lock();
    let endpoint_object_id = state
        .streams
        .iter()
        .find(|slot| slot.occupied && slot.object_id == object_id)
        .ok_or("audio stream not found")?
        .endpoint_object_id;
    let endpoint = state
        .endpoints
        .iter()
        .find(|slot| slot.occupied && slot.info.object_id == endpoint_object_id)
        .ok_or("audio stream endpoint disappeared")?
        .info;
    if config.channels < endpoint.min_channels || config.channels > endpoint.max_channels {
        return Err("stream channel count is outside endpoint capability");
    }
    if config.sample_rate_hz < endpoint.min_rate_hz || config.sample_rate_hz > endpoint.max_rate_hz {
        return Err("stream rate is outside endpoint capability");
    }
    let format = AudioSampleFormat::from_raw(config.sample_format)
        .ok_or("invalid audio sample format")?;
    if endpoint.format_mask & format.mask() == 0 {
        return Err("stream sample format is unsupported by endpoint");
    }
    if endpoint.direction != AudioDirection::Duplex as u32 && endpoint.direction != config.direction {
        return Err("stream direction is incompatible with endpoint");
    }
    let slot = state
        .streams
        .iter_mut()
        .find(|slot| slot.occupied && slot.object_id == object_id)
        .ok_or("audio stream not found")?;
    let mut machine = StreamStateMachine {
        state: slot.state,
        configured: slot.configured,
    };
    machine.configure()?;
    slot.state = machine.state;
    slot.configured = machine.configured;
    slot.config = config;
    slot.frame_position = 0;
    Ok(())
}

pub fn prepare_stream(object_id: u64) -> Result<(), &'static str> {
    transition_stream(object_id, |machine| machine.prepare())
}

pub fn start_stream(object_id: u64) -> Result<(), &'static str> {
    transition_stream(object_id, |machine| machine.start())
}

pub fn drain_stream(object_id: u64) -> Result<(), &'static str> {
    transition_stream(object_id, |machine| machine.drain())
}

pub fn stop_stream(object_id: u64) -> Result<(), &'static str> {
    transition_stream(object_id, |machine| machine.stop())
}

pub fn recover_stream(object_id: u64) -> Result<(), &'static str> {
    transition_stream(object_id, |machine| machine.recover())
}

pub fn fault_stream(object_id: u64) -> Result<(), &'static str> {
    let mut state = STATE.lock();
    let slot = state
        .streams
        .iter_mut()
        .find(|slot| slot.occupied && slot.object_id == object_id)
        .ok_or("audio stream not found")?;
    let mut machine = StreamStateMachine {
        state: slot.state,
        configured: slot.configured,
    };
    machine.fault();
    slot.state = machine.state;
    Ok(())
}

fn transition_stream(
    object_id: u64,
    operation: impl FnOnce(&mut StreamStateMachine) -> Result<(), &'static str>,
) -> Result<(), &'static str> {
    let mut state = STATE.lock();
    let slot = state
        .streams
        .iter_mut()
        .find(|slot| slot.occupied && slot.object_id == object_id)
        .ok_or("audio stream not found")?;
    let mut machine = StreamStateMachine {
        state: slot.state,
        configured: slot.configured,
    };
    operation(&mut machine)?;
    slot.state = machine.state;
    slot.configured = machine.configured;
    Ok(())
}

#[must_use]
pub fn object_generation(kind: AudioObjectKind, object_id: u64) -> Option<u32> {
    let state = STATE.lock();
    match kind {
        AudioObjectKind::Device => state.devices.iter().find(|slot| slot.occupied && slot.info.object_id == object_id).map(|slot| slot.generation),
        AudioObjectKind::Endpoint => state.endpoints.iter().find(|slot| slot.occupied && slot.info.object_id == object_id).map(|slot| slot.generation),
        AudioObjectKind::Stream => state.streams.iter().find(|slot| slot.occupied && slot.object_id == object_id).map(|slot| slot.generation),
        AudioObjectKind::Buffer => state.buffers.iter().find(|slot| slot.occupied && slot.object_id == object_id).map(|slot| slot.generation),
        AudioObjectKind::Clock => state.clocks.iter().find(|slot| slot.occupied && slot.object_id == object_id).map(|slot| slot.generation),
        AudioObjectKind::Event => state.events.iter().find(|slot| slot.occupied && slot.object_id == object_id).map(|slot| slot.generation),
        AudioObjectKind::Fence => state.fences.iter().find(|slot| slot.occupied && slot.object_id == object_id).map(|slot| slot.generation),
    }
}

#[must_use]
pub fn stream_position(object_id: u64) -> Option<StreamPosition> {
    let state = STATE.lock();
    state
        .streams
        .iter()
        .find(|slot| slot.occupied && slot.object_id == object_id)
        .map(|slot| StreamPosition {
            state: slot.state,
            frame_position: slot.frame_position,
        })
}

pub fn advance_stream_position(object_id: u64, frames: u64) -> Result<u64, &'static str> {
    let mut state = STATE.lock();
    let slot = state
        .streams
        .iter_mut()
        .find(|slot| slot.occupied && slot.object_id == object_id)
        .ok_or("audio stream not found")?;
    if !matches!(slot.state, AudioStreamState::Running | AudioStreamState::Draining) {
        return Err("audio stream position advances only while active");
    }
    slot.frame_position = slot
        .frame_position
        .checked_add(frames)
        .ok_or("audio stream frame position overflow")?;
    Ok(slot.frame_position)
}

pub fn create_buffer(
    owner_pid: u64,
    byte_capacity: u32,
    frame_stride_bytes: u32,
    flags: u32,
) -> Result<AudioObjectRef, &'static str> {
    if byte_capacity == 0 || byte_capacity as usize > MAX_ABI_BUFFER_BYTES {
        return Err("audio buffer capacity outside K15.2 bounded pool");
    }
    if frame_stride_bytes == 0 || byte_capacity % frame_stride_bytes != 0 {
        return Err("audio buffer frame stride is invalid");
    }
    if flags & (AUDIO_BUFFER_FLAG_READABLE | AUDIO_BUFFER_FLAG_WRITABLE) == 0 {
        return Err("audio buffer must grant read or write access");
    }
    let mut state = STATE.lock();
    let index = state
        .buffers
        .iter()
        .position(|slot| !slot.occupied)
        .ok_or("ForgeAudio buffer table is full")?;
    let object_id = state.allocate_object_id()?;
    let generation = state.buffers[index].generation.wrapping_add(1).max(1);
    let mut slot = BufferSlot::EMPTY;
    slot.occupied = true;
    slot.object_id = object_id;
    slot.generation = generation;
    slot.owner_pid = owner_pid;
    slot.flags = flags;
    slot.byte_capacity = byte_capacity;
    slot.frame_stride_bytes = frame_stride_bytes;
    state.buffers[index] = slot;
    Ok(AudioObjectRef {
        kind: AudioObjectKind::Buffer,
        object_id,
        generation,
    })
}

pub fn write_buffer(object_id: u64, offset: usize, input: &[u8]) -> Result<usize, &'static str> {
    let mut state = STATE.lock();
    let slot = state
        .buffers
        .iter_mut()
        .find(|slot| slot.occupied && slot.object_id == object_id)
        .ok_or("audio buffer not found")?;
    if slot.flags & AUDIO_BUFFER_FLAG_WRITABLE == 0 {
        return Err("audio buffer is not writable");
    }
    let end = offset
        .checked_add(input.len())
        .ok_or("audio buffer write overflow")?;
    if end > slot.byte_capacity as usize {
        return Err("audio buffer write exceeds capacity");
    }
    slot.bytes[offset..end].copy_from_slice(input);
    slot.committed_bytes = core::cmp::max(slot.committed_bytes, end as u32);
    slot.sequence = slot.sequence.wrapping_add(1);
    Ok(input.len())
}

pub fn read_buffer(object_id: u64, offset: usize, output: &mut [u8]) -> Result<usize, &'static str> {
    let state = STATE.lock();
    let slot = state
        .buffers
        .iter()
        .find(|slot| slot.occupied && slot.object_id == object_id)
        .ok_or("audio buffer not found")?;
    if slot.flags & AUDIO_BUFFER_FLAG_READABLE == 0 {
        return Err("audio buffer is not readable");
    }
    let end = offset
        .checked_add(output.len())
        .ok_or("audio buffer read overflow")?;
    if end > slot.committed_bytes as usize {
        return Err("audio buffer read exceeds committed data");
    }
    output.copy_from_slice(&slot.bytes[offset..end]);
    Ok(output.len())
}

#[must_use]
pub fn buffer_info(object_id: u64) -> Option<AudioBufferInfo> {
    let state = STATE.lock();
    state
        .buffers
        .iter()
        .find(|slot| slot.occupied && slot.object_id == object_id)
        .map(|slot| AudioBufferInfo {
            object_id: slot.object_id,
            generation: slot.generation,
            flags: slot.flags,
            byte_capacity: slot.byte_capacity,
            frame_stride_bytes: slot.frame_stride_bytes,
            frame_capacity: slot.byte_capacity / slot.frame_stride_bytes,
            committed_bytes: slot.committed_bytes,
            sequence: slot.sequence,
        })
}

pub fn create_clock(
    owner_pid: u64,
    rate_numerator: u32,
    rate_denominator: u32,
) -> Result<AudioObjectRef, &'static str> {
    if rate_numerator == 0 || rate_denominator == 0 {
        return Err("audio clock rate must be non-zero");
    }
    let mut state = STATE.lock();
    let index = state
        .clocks
        .iter()
        .position(|slot| !slot.occupied)
        .ok_or("ForgeAudio clock table is full")?;
    let object_id = state.allocate_object_id()?;
    let generation = state.clocks[index].generation.wrapping_add(1).max(1);
    state.clocks[index] = ClockSlot {
        occupied: true,
        object_id,
        generation,
        owner_pid,
        base_tick: scheduler::current_rt_clock_tick(),
        rate_numerator,
        rate_denominator,
    };
    Ok(AudioObjectRef {
        kind: AudioObjectKind::Clock,
        object_id,
        generation,
    })
}

#[must_use]
pub fn clock_snapshot(object_id: u64) -> Option<AudioClockSnapshot> {
    let state = STATE.lock();
    let slot = state
        .clocks
        .iter()
        .find(|slot| slot.occupied && slot.object_id == object_id)?;
    let now = scheduler::current_rt_clock_tick();
    let elapsed = now.saturating_sub(slot.base_tick);
    let tick_hz = u64::from(scheduler::FORGEAUDIO_RT_TICK_HZ);
    let nanoseconds = elapsed.saturating_mul(1_000_000_000) / tick_hz;
    let frame_position = elapsed
        .saturating_mul(u64::from(slot.rate_numerator))
        / tick_hz.saturating_mul(u64::from(slot.rate_denominator));
    Some(AudioClockSnapshot {
        object_id: slot.object_id,
        generation: slot.generation,
        flags: AUDIO_CLOCK_FLAG_MONOTONIC,
        tick: now,
        nanoseconds,
        frame_position,
        rate_numerator: slot.rate_numerator,
        rate_denominator: slot.rate_denominator,
    })
}

pub fn create_event(owner_pid: u64) -> Result<AudioObjectRef, &'static str> {
    let mut state = STATE.lock();
    let index = state
        .events
        .iter()
        .position(|slot| !slot.occupied)
        .ok_or("ForgeAudio event table is full")?;
    let object_id = state.allocate_object_id()?;
    let generation = state.events[index].generation.wrapping_add(1).max(1);
    let mut slot = EventSlot::EMPTY;
    slot.occupied = true;
    slot.object_id = object_id;
    slot.generation = generation;
    slot.owner_pid = owner_pid;
    state.events[index] = slot;
    Ok(AudioObjectRef {
        kind: AudioObjectKind::Event,
        object_id,
        generation,
    })
}

pub fn push_event(
    event_object_id: u64,
    subject_object_id: u64,
    kind: AudioEventKind,
    code: u32,
    value0: u64,
    value1: u64,
) -> Result<u64, &'static str> {
    let mut state = STATE.lock();
    let slot = state
        .events
        .iter_mut()
        .find(|slot| slot.occupied && slot.object_id == event_object_id)
        .ok_or("ForgeAudio event object not found")?;
    if slot.count == EVENT_QUEUE_DEPTH {
        return Err("ForgeAudio event queue is full");
    }
    let sequence = slot.next_sequence;
    slot.next_sequence = slot.next_sequence.wrapping_add(1).max(1);
    let index = (slot.head + slot.count) % EVENT_QUEUE_DEPTH;
    slot.records[index] = AudioEventRecord {
        sequence,
        object_id: subject_object_id,
        timestamp_tick: scheduler::current_rt_clock_tick(),
        kind: kind as u32,
        code,
        value0,
        value1,
    };
    slot.count += 1;
    Ok(sequence)
}

pub fn poll_event(event_object_id: u64) -> Result<Option<AudioEventRecord>, &'static str> {
    let mut state = STATE.lock();
    let slot = state
        .events
        .iter_mut()
        .find(|slot| slot.occupied && slot.object_id == event_object_id)
        .ok_or("ForgeAudio event object not found")?;
    if slot.count == 0 {
        return Ok(None);
    }
    let record = slot.records[slot.head];
    slot.records[slot.head] = EMPTY_EVENT;
    slot.head = (slot.head + 1) % EVENT_QUEUE_DEPTH;
    slot.count -= 1;
    Ok(Some(record))
}

pub fn create_fence(owner_pid: u64, target_value: u64) -> Result<AudioObjectRef, &'static str> {
    if target_value == 0 {
        return Err("ForgeAudio fence target must be non-zero");
    }
    let mut state = STATE.lock();
    let index = state
        .fences
        .iter()
        .position(|slot| !slot.occupied)
        .ok_or("ForgeAudio fence table is full")?;
    let object_id = state.allocate_object_id()?;
    let generation = state.fences[index].generation.wrapping_add(1).max(1);
    state.fences[index] = FenceSlot {
        occupied: true,
        object_id,
        generation,
        owner_pid,
        target_value,
        completed_value: 0,
        sequence: 1,
    };
    Ok(AudioObjectRef {
        kind: AudioObjectKind::Fence,
        object_id,
        generation,
    })
}

pub fn signal_fence(object_id: u64, completed_value: u64) -> Result<bool, &'static str> {
    let mut state = STATE.lock();
    let slot = state
        .fences
        .iter_mut()
        .find(|slot| slot.occupied && slot.object_id == object_id)
        .ok_or("ForgeAudio fence not found")?;
    if completed_value < slot.completed_value {
        return Err("ForgeAudio fence completion cannot move backwards");
    }
    slot.completed_value = completed_value;
    slot.sequence = slot.sequence.wrapping_add(1);
    Ok(slot.completed_value >= slot.target_value)
}

#[must_use]
pub fn fence_info(object_id: u64) -> Option<AudioFenceInfo> {
    let state = STATE.lock();
    state
        .fences
        .iter()
        .find(|slot| slot.occupied && slot.object_id == object_id)
        .map(|slot| AudioFenceInfo {
            object_id: slot.object_id,
            generation: slot.generation,
            flags: if slot.completed_value >= slot.target_value {
                AUDIO_FENCE_FLAG_SIGNALED
            } else {
                0
            },
            target_value: slot.target_value,
            completed_value: slot.completed_value,
            sequence: slot.sequence,
        })
}

pub fn release_object(kind: AudioObjectKind, object_id: u64, owner_pid: u64) -> Result<(), &'static str> {
    let mut state = STATE.lock();
    match kind {
        AudioObjectKind::Device => {
            let slot = state
                .devices
                .iter_mut()
                .find(|slot| slot.occupied && slot.info.object_id == object_id)
                .ok_or("audio device not found")?;
            if slot.open_references == 0 {
                return Err("audio device reference underflow");
            }
            slot.open_references -= 1;
        }
        AudioObjectKind::Endpoint => return Err("audio endpoints are driver-owned objects"),
        AudioObjectKind::Stream => release_stream(&mut state, object_id, owner_pid)?,
        AudioObjectKind::Buffer => release_buffer(&mut state, object_id, owner_pid)?,
        AudioObjectKind::Clock => release_clock(&mut state, object_id, owner_pid)?,
        AudioObjectKind::Event => release_event(&mut state, object_id, owner_pid)?,
        AudioObjectKind::Fence => release_fence(&mut state, object_id, owner_pid)?,
    }
    Ok(())
}

fn release_stream(state: &mut ForgeAudioState, object_id: u64, owner_pid: u64) -> Result<(), &'static str> {
    let slot = state
        .streams
        .iter_mut()
        .find(|slot| slot.occupied && slot.object_id == object_id)
        .ok_or("audio stream not found")?;
    if slot.owner_pid != owner_pid {
        return Err("audio stream owner mismatch");
    }
    let generation = slot.generation;
    *slot = StreamSlot::EMPTY;
    slot.generation = generation;
    Ok(())
}

fn release_buffer(state: &mut ForgeAudioState, object_id: u64, owner_pid: u64) -> Result<(), &'static str> {
    let slot = state
        .buffers
        .iter_mut()
        .find(|slot| slot.occupied && slot.object_id == object_id)
        .ok_or("audio buffer not found")?;
    if slot.owner_pid != owner_pid {
        return Err("audio buffer owner mismatch");
    }
    let generation = slot.generation;
    for byte in &mut slot.bytes[..slot.byte_capacity as usize] {
        *byte = 0;
    }
    *slot = BufferSlot::EMPTY;
    slot.generation = generation;
    Ok(())
}

fn release_clock(state: &mut ForgeAudioState, object_id: u64, owner_pid: u64) -> Result<(), &'static str> {
    let slot = state
        .clocks
        .iter_mut()
        .find(|slot| slot.occupied && slot.object_id == object_id)
        .ok_or("audio clock not found")?;
    if slot.owner_pid != owner_pid {
        return Err("audio clock owner mismatch");
    }
    let generation = slot.generation;
    *slot = ClockSlot::EMPTY;
    slot.generation = generation;
    Ok(())
}

fn release_event(state: &mut ForgeAudioState, object_id: u64, owner_pid: u64) -> Result<(), &'static str> {
    let slot = state
        .events
        .iter_mut()
        .find(|slot| slot.occupied && slot.object_id == object_id)
        .ok_or("audio event not found")?;
    if slot.owner_pid != owner_pid {
        return Err("audio event owner mismatch");
    }
    let generation = slot.generation;
    *slot = EventSlot::EMPTY;
    slot.generation = generation;
    Ok(())
}

fn release_fence(state: &mut ForgeAudioState, object_id: u64, owner_pid: u64) -> Result<(), &'static str> {
    let slot = state
        .fences
        .iter_mut()
        .find(|slot| slot.occupied && slot.object_id == object_id)
        .ok_or("audio fence not found")?;
    if slot.owner_pid != owner_pid {
        return Err("audio fence owner mismatch");
    }
    let generation = slot.generation;
    *slot = FenceSlot::EMPTY;
    slot.generation = generation;
    Ok(())
}

pub fn run_abi_self_test() -> Result<(), &'static str> {
    let info = abi_info();
    if info.current_version != FORGEAUDIO_ABI_VERSION || info.features != FORGEAUDIO_FEATURES_V1 {
        return Err("ForgeAudio ABI version/features mismatch");
    }

    if device_count() != 0 || enumerate_device(0).is_some() {
        return Err("K15.2 must not invent an audio hardware device");
    }
    if open_device(0xDEAD_BEEF).is_ok() {
        return Err("nonexistent audio device unexpectedly opened");
    }
    serial::println(format_args!(
        "[K15ABI] hardware registry honest: devices=0 qemu_deferred=true fake_devices=false"
    ));

    let mut machine = StreamStateMachine::new();
    if machine.start().is_ok() {
        return Err("unprepared audio stream unexpectedly started");
    }
    machine.configure()?;
    machine.prepare()?;
    machine.start()?;
    machine.drain()?;
    machine.stop()?;
    machine.configure()?;
    machine.fault();
    machine.recover()?;
    if machine.state != AudioStreamState::Configured {
        return Err("audio stream recovery state mismatch");
    }
    serial::println(format_args!(
        "[K15ABI] stream lifecycle state machine qualified: illegal_start_rejected=true recover=true"
    ));

    const TEST_OWNER: u64 = 0x15_0002;
    let buffer = create_buffer(
        TEST_OWNER,
        256,
        4,
        AUDIO_BUFFER_FLAG_READABLE | AUDIO_BUFFER_FLAG_WRITABLE,
    )?;
    let pattern: [u8; 16] = [
        0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE,
        0xEF, 0xCD, 0xAB, 0x89, 0x67, 0x45, 0x23, 0x01,
    ];
    if write_buffer(buffer.object_id, 32, &pattern)? != pattern.len() {
        return Err("audio buffer write count mismatch");
    }
    let mut output = [0u8; 16];
    if read_buffer(buffer.object_id, 32, &mut output)? != output.len() || output != pattern {
        return Err("audio buffer readback mismatch");
    }
    let buffer_state = buffer_info(buffer.object_id).ok_or("audio buffer info missing")?;
    if buffer_state.committed_bytes != 48 || buffer_state.frame_capacity != 64 {
        return Err("audio buffer accounting mismatch");
    }
    serial::println(format_args!(
        "[K15ABI] real bounded buffer qualified: bytes={} stride={} frames={} sequence={}",
        buffer_state.byte_capacity,
        buffer_state.frame_stride_bytes,
        buffer_state.frame_capacity,
        buffer_state.sequence
    ));

    let clock = create_clock(TEST_OWNER, 48_000, 1)?;
    let clock_state = clock_snapshot(clock.object_id).ok_or("audio clock snapshot missing")?;
    if clock_state.rate_numerator != 48_000 || clock_state.rate_denominator != 1 {
        return Err("audio clock rate mismatch");
    }
    serial::println(format_args!(
        "[K15ABI] monotonic audio clock qualified: tick={} ns={} frame={} rate={}/{}",
        clock_state.tick,
        clock_state.nanoseconds,
        clock_state.frame_position,
        clock_state.rate_numerator,
        clock_state.rate_denominator
    ));

    let event = create_event(TEST_OWNER)?;
    let sequence = push_event(
        event.object_id,
        buffer.object_id,
        AudioEventKind::BufferPeriod,
        0x152,
        64,
        128,
    )?;
    let record = poll_event(event.object_id)?.ok_or("audio event was not queued")?;
    if record.sequence != sequence
        || record.object_id != buffer.object_id
        || record.kind != AudioEventKind::BufferPeriod as u32
        || poll_event(event.object_id)?.is_some()
    {
        return Err("audio event FIFO contract mismatch");
    }
    serial::println(format_args!(
        "[K15ABI] bounded event queue qualified: sequence={} drained=true",
        sequence
    ));

    let fence = create_fence(TEST_OWNER, 4)?;
    if signal_fence(fence.object_id, 3)? {
        return Err("audio fence signaled before target");
    }
    if !signal_fence(fence.object_id, 4)? {
        return Err("audio fence did not signal at target");
    }
    let fence_state = fence_info(fence.object_id).ok_or("audio fence info missing")?;
    if fence_state.flags & AUDIO_FENCE_FLAG_SIGNALED == 0 || fence_state.completed_value != 4 {
        return Err("audio fence state mismatch");
    }
    serial::println(format_args!(
        "[K15ABI] monotonic fence qualified: target={} completed={} sequence={}",
        fence_state.target_value,
        fence_state.completed_value,
        fence_state.sequence
    ));

    release_object(AudioObjectKind::Fence, fence.object_id, TEST_OWNER)?;
    release_object(AudioObjectKind::Event, event.object_id, TEST_OWNER)?;
    release_object(AudioObjectKind::Clock, clock.object_id, TEST_OWNER)?;
    release_object(AudioObjectKind::Buffer, buffer.object_id, TEST_OWNER)?;

    if buffer_info(buffer.object_id).is_some()
        || clock_snapshot(clock.object_id).is_some()
        || fence_info(fence.object_id).is_some()
    {
        return Err("ForgeAudio object remained live after release");
    }

    serial::println(format_args!(
        "[K15OK] K15.2 ForgeAudio kernel ABI qualified: ABI=v{} device+endpoint+stream+buffer+clock+event+fence lifecycle real bounded=true fake_devices=false",
        FORGEAUDIO_ABI_VERSION
    ));
    Ok(())
}

// K15.6 ForgeAudioD ownership introspection. This is a kernel-side validation
// helper for the privileged userspace audio server; it does not change the
// frozen K15.2 object lifecycle or expose object tables to ordinary clients.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ForgeAudioServerOwnershipSnapshot {
    pub streams: u32,
    pub playback_streams: u32,
    pub capture_streams: u32,
    pub prepared_streams: u32,
    pub buffers: u32,
    pub clocks: u32,
    pub events: u32,
    pub fences: u32,
}

#[must_use]
pub fn server_ownership_snapshot(
    owner_pid: u64,
    device_object_id: u64,
) -> Option<ForgeAudioServerOwnershipSnapshot> {
    let state = STATE.lock();
    if !state
        .devices
        .iter()
        .any(|slot| slot.occupied && slot.info.object_id == device_object_id)
    {
        return None;
    }

    let mut snapshot = ForgeAudioServerOwnershipSnapshot::default();
    for slot in state.streams.iter().filter(|slot| {
        slot.occupied && slot.owner_pid == owner_pid && slot.device_object_id == device_object_id
    }) {
        snapshot.streams = snapshot.streams.saturating_add(1);
        match AudioDirection::from_raw(slot.config.direction) {
            Some(AudioDirection::Playback) => {
                snapshot.playback_streams = snapshot.playback_streams.saturating_add(1)
            }
            Some(AudioDirection::Capture) => {
                snapshot.capture_streams = snapshot.capture_streams.saturating_add(1)
            }
            Some(AudioDirection::Duplex) | None => {}
        }
        if matches!(
            slot.state,
            AudioStreamState::Prepared | AudioStreamState::Running | AudioStreamState::Draining
        ) {
            snapshot.prepared_streams = snapshot.prepared_streams.saturating_add(1);
        }
    }
    snapshot.buffers = state
        .buffers
        .iter()
        .filter(|slot| slot.occupied && slot.owner_pid == owner_pid)
        .count() as u32;
    snapshot.clocks = state
        .clocks
        .iter()
        .filter(|slot| slot.occupied && slot.owner_pid == owner_pid)
        .count() as u32;
    snapshot.events = state
        .events
        .iter()
        .filter(|slot| slot.occupied && slot.owner_pid == owner_pid)
        .count() as u32;
    snapshot.fences = state
        .fences
        .iter()
        .filter(|slot| slot.occupied && slot.owner_pid == owner_pid)
        .count() as u32;
    Some(snapshot)
}
