#![no_std]

//! Titanweave K15 ForgeAudio stable kernel/userspace ABI.
//!
//! This crate is shared by WeaveCore and userspace so the wire layout cannot
//! drift between the two sides of the system call boundary.

pub const FORGEAUDIO_ABI_VERSION: u32 = 1;
pub const FORGEAUDIO_ABI_MIN_COMPATIBLE_VERSION: u32 = 1;

pub const FORGEAUDIO_FEATURE_DEVICE_OBJECTS: u32 = 1 << 0;
pub const FORGEAUDIO_FEATURE_ENDPOINT_OBJECTS: u32 = 1 << 1;
pub const FORGEAUDIO_FEATURE_STREAM_OBJECTS: u32 = 1 << 2;
pub const FORGEAUDIO_FEATURE_BUFFER_OBJECTS: u32 = 1 << 3;
pub const FORGEAUDIO_FEATURE_CLOCK_OBJECTS: u32 = 1 << 4;
pub const FORGEAUDIO_FEATURE_EVENT_OBJECTS: u32 = 1 << 5;
pub const FORGEAUDIO_FEATURE_FENCE_OBJECTS: u32 = 1 << 6;
pub const FORGEAUDIO_FEATURES_V1: u32 = FORGEAUDIO_FEATURE_DEVICE_OBJECTS
    | FORGEAUDIO_FEATURE_ENDPOINT_OBJECTS
    | FORGEAUDIO_FEATURE_STREAM_OBJECTS
    | FORGEAUDIO_FEATURE_BUFFER_OBJECTS
    | FORGEAUDIO_FEATURE_CLOCK_OBJECTS
    | FORGEAUDIO_FEATURE_EVENT_OBJECTS
    | FORGEAUDIO_FEATURE_FENCE_OBJECTS;

pub const AUDIO_DEVICE_FLAG_PLAYBACK: u32 = 1 << 0;
pub const AUDIO_DEVICE_FLAG_CAPTURE: u32 = 1 << 1;
pub const AUDIO_DEVICE_FLAG_FULL_DUPLEX: u32 = 1 << 2;
pub const AUDIO_DEVICE_FLAG_HOTPLUGGABLE: u32 = 1 << 3;
pub const AUDIO_DEVICE_FLAG_CLOCK_MASTER: u32 = 1 << 4;

/// Native PCI High Definition Audio controller backend.
pub const AUDIO_BACKEND_HDA: u8 = 1;

pub const AUDIO_ENDPOINT_FLAG_DEFAULT: u32 = 1 << 0;
pub const AUDIO_ENDPOINT_FLAG_DIGITAL: u32 = 1 << 1;
pub const AUDIO_ENDPOINT_FLAG_HEADPHONES: u32 = 1 << 2;
pub const AUDIO_ENDPOINT_FLAG_MICROPHONE: u32 = 1 << 3;
pub const AUDIO_ENDPOINT_FLAG_LINE_LEVEL: u32 = 1 << 4;

pub const AUDIO_STREAM_FLAG_EXCLUSIVE: u32 = 1 << 0;
pub const AUDIO_STREAM_FLAG_LOW_LATENCY: u32 = 1 << 1;
pub const AUDIO_STREAM_FLAG_EVENT_DRIVEN: u32 = 1 << 2;

pub const AUDIO_BUFFER_FLAG_READABLE: u32 = 1 << 0;
pub const AUDIO_BUFFER_FLAG_WRITABLE: u32 = 1 << 1;
pub const AUDIO_BUFFER_FLAG_DMA_CAPABLE: u32 = 1 << 2;

pub const AUDIO_FENCE_FLAG_SIGNALED: u32 = 1 << 0;
pub const AUDIO_CLOCK_FLAG_MONOTONIC: u32 = 1 << 0;
pub const AUDIO_CLOCK_FLAG_DEVICE: u32 = 1 << 1;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioObjectKind {
    Device = 1,
    Endpoint = 2,
    Stream = 3,
    Buffer = 4,
    Clock = 5,
    Event = 6,
    Fence = 7,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioDirection {
    Playback = 1,
    Capture = 2,
    Duplex = 3,
}

impl AudioDirection {
    #[must_use]
    pub const fn from_raw(value: u32) -> Option<Self> {
        match value {
            1 => Some(Self::Playback),
            2 => Some(Self::Capture),
            3 => Some(Self::Duplex),
            _ => None,
        }
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioSampleFormat {
    S16 = 1,
    S24In32 = 2,
    S32 = 3,
    F32 = 4,
}

impl AudioSampleFormat {
    #[must_use]
    pub const fn from_raw(value: u32) -> Option<Self> {
        match value {
            1 => Some(Self::S16),
            2 => Some(Self::S24In32),
            3 => Some(Self::S32),
            4 => Some(Self::F32),
            _ => None,
        }
    }

    #[must_use]
    pub const fn bytes_per_sample(self) -> u32 {
        match self {
            Self::S16 => 2,
            Self::S24In32 | Self::S32 | Self::F32 => 4,
        }
    }

    #[must_use]
    pub const fn mask(self) -> u32 {
        1u32 << ((self as u32) - 1)
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioStreamState {
    Created = 1,
    Configured = 2,
    Prepared = 3,
    Running = 4,
    Draining = 5,
    Stopped = 6,
    Faulted = 7,
    Closed = 8,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioEventKind {
    StreamStarted = 1,
    StreamStopped = 2,
    StreamDrained = 3,
    StreamFaulted = 4,
    BufferPeriod = 5,
    FenceSignaled = 6,
    DeviceAdded = 7,
    DeviceRemoved = 8,
    ClockDiscontinuity = 9,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioControlOp {
    OpenDevice = 1,
    OpenStream = 2,
    ConfigureStream = 3,
    PrepareStream = 4,
    StartStream = 5,
    StopStream = 6,
    DrainStream = 7,
    RecoverStream = 8,
    QueryPosition = 9,
    CreateBuffer = 10,
    CreateClock = 11,
    CreateEvent = 12,
    CreateFence = 13,
    PollEvent = 14,
    QueryFence = 15,
    CloseObject = 16,
}

impl AudioControlOp {
    #[must_use]
    pub const fn from_raw(value: u32) -> Option<Self> {
        match value {
            1 => Some(Self::OpenDevice),
            2 => Some(Self::OpenStream),
            3 => Some(Self::ConfigureStream),
            4 => Some(Self::PrepareStream),
            5 => Some(Self::StartStream),
            6 => Some(Self::StopStream),
            7 => Some(Self::DrainStream),
            8 => Some(Self::RecoverStream),
            9 => Some(Self::QueryPosition),
            10 => Some(Self::CreateBuffer),
            11 => Some(Self::CreateClock),
            12 => Some(Self::CreateEvent),
            13 => Some(Self::CreateFence),
            14 => Some(Self::PollEvent),
            15 => Some(Self::QueryFence),
            16 => Some(Self::CloseObject),
            _ => None,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioAbiInfo {
    pub current_version: u32,
    pub minimum_compatible_version: u32,
    pub features: u32,
    pub reserved: u32,
    pub max_devices: u32,
    pub max_endpoints: u32,
    pub max_streams: u32,
    pub max_buffers: u32,
    pub max_clocks: u32,
    pub max_events: u32,
    pub max_fences: u32,
    pub reserved2: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioDeviceInfo {
    pub object_id: u64,
    pub generation: u32,
    pub flags: u32,
    pub vendor_id: u16,
    pub device_id: u16,
    pub subsystem_vendor_id: u16,
    pub subsystem_device_id: u16,
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub backend_kind: u8,
    pub endpoint_count: u32,
    pub clock_domain: u32,
    pub name: [u8; 32],
    pub reserved2: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioEndpointInfo {
    pub object_id: u64,
    pub device_object_id: u64,
    pub generation: u32,
    pub direction: u32,
    pub flags: u32,
    pub min_channels: u16,
    pub max_channels: u16,
    pub min_rate_hz: u32,
    pub max_rate_hz: u32,
    pub format_mask: u32,
    pub reserved: u32,
    pub name: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioStreamConfig {
    pub abi_version: u32,
    pub flags: u32,
    pub direction: u32,
    pub sample_format: u32,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub reserved0: u16,
    pub period_frames: u32,
    pub buffer_frames: u32,
    pub reserved1: u64,
}

impl AudioStreamConfig {
    #[must_use]
    pub fn frame_stride_bytes(&self) -> Option<u32> {
        let Some(format) = AudioSampleFormat::from_raw(self.sample_format) else {
            return None;
        };
        format.bytes_per_sample().checked_mul(self.channels as u32)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.abi_version != FORGEAUDIO_ABI_VERSION {
            return Err("unsupported ForgeAudio ABI version");
        }
        if AudioDirection::from_raw(self.direction).is_none() {
            return Err("invalid audio stream direction");
        }
        if AudioSampleFormat::from_raw(self.sample_format).is_none() {
            return Err("invalid audio sample format");
        }
        if self.sample_rate_hz < 8_000 || self.sample_rate_hz > 384_000 {
            return Err("audio sample rate outside ABI bounds");
        }
        if self.channels == 0 || self.channels > 64 {
            return Err("audio channel count outside ABI bounds");
        }
        if self.period_frames == 0 || self.buffer_frames < self.period_frames {
            return Err("invalid audio period/buffer frame geometry");
        }
        if self.buffer_frames % self.period_frames != 0 {
            return Err("audio buffer must contain a whole number of periods");
        }
        if self.frame_stride_bytes().is_none() {
            return Err("audio frame stride overflow");
        }
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioBufferInfo {
    pub object_id: u64,
    pub generation: u32,
    pub flags: u32,
    pub byte_capacity: u32,
    pub frame_stride_bytes: u32,
    pub frame_capacity: u32,
    pub committed_bytes: u32,
    pub sequence: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioClockSnapshot {
    pub object_id: u64,
    pub generation: u32,
    pub flags: u32,
    pub tick: u64,
    pub nanoseconds: u64,
    pub frame_position: u64,
    pub rate_numerator: u32,
    pub rate_denominator: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioEventRecord {
    pub sequence: u64,
    pub object_id: u64,
    pub timestamp_tick: u64,
    pub kind: u32,
    pub code: u32,
    pub value0: u64,
    pub value1: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioFenceInfo {
    pub object_id: u64,
    pub generation: u32,
    pub flags: u32,
    pub target_value: u64,
    pub completed_value: u64,
    pub sequence: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioControlRequest {
    pub abi_version: u32,
    pub operation: u32,
    pub handle: u32,
    pub flags: u32,
    pub object_id: u64,
    pub arg0: u64,
    pub arg1: u64,
    pub arg2: u64,
    pub arg3: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioControlResponse {
    pub abi_version: u32,
    pub object_kind: u32,
    pub handle: u32,
    pub state: u32,
    pub object_id: u64,
    pub generation: u32,
    pub flags: u32,
    pub value0: u64,
    pub value1: u64,
    pub value2: u64,
    pub value3: u64,
}

const _: [(); 48] = [(); core::mem::size_of::<AudioAbiInfo>()];
const _: [(); 72] = [(); core::mem::size_of::<AudioDeviceInfo>()];
const _: [(); 80] = [(); core::mem::size_of::<AudioEndpointInfo>()];
const _: [(); 40] = [(); core::mem::size_of::<AudioStreamConfig>()];
const _: [(); 40] = [(); core::mem::size_of::<AudioBufferInfo>()];
const _: [(); 48] = [(); core::mem::size_of::<AudioClockSnapshot>()];
const _: [(); 48] = [(); core::mem::size_of::<AudioEventRecord>()];
const _: [(); 40] = [(); core::mem::size_of::<AudioFenceInfo>()];
const _: [(); 56] = [(); core::mem::size_of::<AudioControlRequest>()];
const _: [(); 64] = [(); core::mem::size_of::<AudioControlResponse>()];
