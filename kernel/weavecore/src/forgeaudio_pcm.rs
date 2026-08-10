//! K15.5 ForgeAudio PCM Format Engine.
//!
//! Backend-neutral, bounded PCM representation and negotiation.  This gate
//! does not mix, resample, or own userspace routing (those belong to later
//! K15 gates).  It defines canonical sample/container semantics, channel maps,
//! interleaved/planar transforms, supported-rate negotiation, HDA PCM
//! capability parsing, HDA stream-format encode/decode, and exact DMA period
//! geometry.  All required paths are allocation-free and bounded.

use crate::{forgeaudio, forgeaudio_dma::{MAX_AUDIO_DMA_PERIODS, MAX_AUDIO_DMA_RING_BYTES}, serial};
use titanweave_forgeaudio_abi::{
    AudioDirection, AudioEndpointInfo, AudioSampleFormat, AudioStreamConfig,
    AUDIO_BACKEND_HDA, FORGEAUDIO_ABI_VERSION,
};

pub const FORGEAUDIO_PCM_ENGINE_VERSION: u32 = 1;
pub const MAX_PCM_CHANNELS: usize = 16;
pub const MAX_PCM_CONVERSION_FRAMES: usize = 2048;
pub const HDA_PCM_RATE_BITS: u16 = 0x0fff;
pub const HDA_PCM_BITS_8: u32 = 1 << 16;
pub const HDA_PCM_BITS_16: u32 = 1 << 17;
pub const HDA_PCM_BITS_20: u32 = 1 << 18;
pub const HDA_PCM_BITS_24: u32 = 1 << 19;
pub const HDA_PCM_BITS_32: u32 = 1 << 20;
pub const HDA_STREAM_FORMAT_PCM: u32 = 1 << 0;
pub const HDA_STREAM_FORMAT_FLOAT32: u32 = 1 << 1;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcmStorageLayout {
    Interleaved = 1,
    Planar = 2,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RatePolicy {
    Exact = 1,
    Nearest = 2,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelPosition {
    Unused = 0,
    FrontLeft = 1,
    FrontRight = 2,
    FrontCenter = 3,
    LowFrequency = 4,
    RearLeft = 5,
    RearRight = 6,
    SideLeft = 7,
    SideRight = 8,
    FrontLeftCenter = 9,
    FrontRightCenter = 10,
    RearCenter = 11,
    TopCenter = 12,
    TopFrontLeft = 13,
    TopFrontRight = 14,
    TopRearLeft = 15,
    TopRearRight = 16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelMap {
    pub channels: u8,
    pub positions: [ChannelPosition; MAX_PCM_CHANNELS],
}

impl ChannelMap {
    pub const EMPTY: Self = Self {
        channels: 0,
        positions: [ChannelPosition::Unused; MAX_PCM_CHANNELS],
    };

    pub fn canonical(channels: u8) -> Result<Self, &'static str> {
        if channels == 0 || usize::from(channels) > MAX_PCM_CHANNELS {
            return Err("PCM channel count is outside canonical map bounds");
        }
        let mut positions = [ChannelPosition::Unused; MAX_PCM_CHANNELS];
        match channels {
            1 => positions[0] = ChannelPosition::FrontCenter,
            2 => {
                positions[0] = ChannelPosition::FrontLeft;
                positions[1] = ChannelPosition::FrontRight;
            }
            3 => {
                positions[0] = ChannelPosition::FrontLeft;
                positions[1] = ChannelPosition::FrontRight;
                positions[2] = ChannelPosition::FrontCenter;
            }
            4 => {
                positions[0] = ChannelPosition::FrontLeft;
                positions[1] = ChannelPosition::FrontRight;
                positions[2] = ChannelPosition::RearLeft;
                positions[3] = ChannelPosition::RearRight;
            }
            5 => {
                positions[0] = ChannelPosition::FrontLeft;
                positions[1] = ChannelPosition::FrontRight;
                positions[2] = ChannelPosition::FrontCenter;
                positions[3] = ChannelPosition::RearLeft;
                positions[4] = ChannelPosition::RearRight;
            }
            6 => {
                positions[0] = ChannelPosition::FrontLeft;
                positions[1] = ChannelPosition::FrontRight;
                positions[2] = ChannelPosition::FrontCenter;
                positions[3] = ChannelPosition::LowFrequency;
                positions[4] = ChannelPosition::RearLeft;
                positions[5] = ChannelPosition::RearRight;
            }
            7 => {
                positions[0] = ChannelPosition::FrontLeft;
                positions[1] = ChannelPosition::FrontRight;
                positions[2] = ChannelPosition::FrontCenter;
                positions[3] = ChannelPosition::LowFrequency;
                positions[4] = ChannelPosition::RearCenter;
                positions[5] = ChannelPosition::SideLeft;
                positions[6] = ChannelPosition::SideRight;
            }
            8 => {
                positions[0] = ChannelPosition::FrontLeft;
                positions[1] = ChannelPosition::FrontRight;
                positions[2] = ChannelPosition::FrontCenter;
                positions[3] = ChannelPosition::LowFrequency;
                positions[4] = ChannelPosition::RearLeft;
                positions[5] = ChannelPosition::RearRight;
                positions[6] = ChannelPosition::SideLeft;
                positions[7] = ChannelPosition::SideRight;
            }
            _ => {
                const ORDER: [ChannelPosition; MAX_PCM_CHANNELS] = [
                    ChannelPosition::FrontLeft,
                    ChannelPosition::FrontRight,
                    ChannelPosition::FrontCenter,
                    ChannelPosition::LowFrequency,
                    ChannelPosition::RearLeft,
                    ChannelPosition::RearRight,
                    ChannelPosition::SideLeft,
                    ChannelPosition::SideRight,
                    ChannelPosition::FrontLeftCenter,
                    ChannelPosition::FrontRightCenter,
                    ChannelPosition::RearCenter,
                    ChannelPosition::TopCenter,
                    ChannelPosition::TopFrontLeft,
                    ChannelPosition::TopFrontRight,
                    ChannelPosition::TopRearLeft,
                    ChannelPosition::TopRearRight,
                ];
                let mut index = 0usize;
                while index < channels as usize {
                    positions[index] = ORDER[index];
                    index += 1;
                }
            }
        }
        Ok(Self { channels, positions })
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.channels == 0 || usize::from(self.channels) > MAX_PCM_CHANNELS {
            return Err("PCM channel map count is invalid");
        }
        for index in 0..usize::from(self.channels) {
            let position = self.positions[index];
            if position == ChannelPosition::Unused {
                return Err("PCM channel map contains unused active slot");
            }
            for prior in 0..index {
                if self.positions[prior] == position {
                    return Err("PCM channel map contains duplicate channel position");
                }
            }
        }
        Ok(())
    }

    fn index_of(&self, position: ChannelPosition) -> Option<usize> {
        (0..usize::from(self.channels)).find(|&index| self.positions[index] == position)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HdaRateSpec {
    pub rate_hz: u32,
    pub capability_bit: u8,
    pub base_44k: bool,
    pub multiplier: u8,
    pub divisor: u8,
}

// HDA rate bits 0..11 and stream-format base/multiplier/divisor encodings.
pub const HDA_RATES: [HdaRateSpec; 12] = [
    HdaRateSpec { rate_hz: 8_000, capability_bit: 0, base_44k: false, multiplier: 1, divisor: 6 },
    HdaRateSpec { rate_hz: 11_025, capability_bit: 1, base_44k: true, multiplier: 1, divisor: 4 },
    HdaRateSpec { rate_hz: 16_000, capability_bit: 2, base_44k: false, multiplier: 1, divisor: 3 },
    HdaRateSpec { rate_hz: 22_050, capability_bit: 3, base_44k: true, multiplier: 1, divisor: 2 },
    HdaRateSpec { rate_hz: 32_000, capability_bit: 4, base_44k: false, multiplier: 2, divisor: 3 },
    HdaRateSpec { rate_hz: 44_100, capability_bit: 5, base_44k: true, multiplier: 1, divisor: 1 },
    HdaRateSpec { rate_hz: 48_000, capability_bit: 6, base_44k: false, multiplier: 1, divisor: 1 },
    HdaRateSpec { rate_hz: 88_200, capability_bit: 7, base_44k: true, multiplier: 2, divisor: 1 },
    HdaRateSpec { rate_hz: 96_000, capability_bit: 8, base_44k: false, multiplier: 2, divisor: 1 },
    HdaRateSpec { rate_hz: 176_400, capability_bit: 9, base_44k: true, multiplier: 4, divisor: 1 },
    HdaRateSpec { rate_hz: 192_000, capability_bit: 10, base_44k: false, multiplier: 4, divisor: 1 },
    HdaRateSpec { rate_hz: 384_000, capability_bit: 11, base_44k: false, multiplier: 8, divisor: 1 },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PcmCapabilities {
    pub rate_mask: u16,
    pub sample_format_mask: u32,
    pub min_channels: u8,
    pub max_channels: u8,
    pub interleaved: bool,
    pub planar: bool,
    pub hda_pcm: bool,
    pub hda_float32: bool,
}

impl PcmCapabilities {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.rate_mask & HDA_PCM_RATE_BITS == 0 {
            return Err("PCM capability set contains no supported rates");
        }
        if self.sample_format_mask == 0 {
            return Err("PCM capability set contains no sample formats");
        }
        if self.min_channels == 0
            || self.max_channels < self.min_channels
            || usize::from(self.max_channels) > MAX_PCM_CHANNELS
        {
            return Err("PCM capability channel range is invalid");
        }
        if !self.interleaved && !self.planar {
            return Err("PCM capability set contains no storage layout");
        }
        Ok(())
    }

    pub fn from_hda_parameters(
        pcm_size_rates: u32,
        stream_formats: u32,
        max_channels: u8,
    ) -> Result<Self, &'static str> {
        let pcm = stream_formats & HDA_STREAM_FORMAT_PCM != 0;
        let float32 = stream_formats & HDA_STREAM_FORMAT_FLOAT32 != 0;
        if !pcm && !float32 {
            return Err("HDA converter advertises neither PCM nor Float32 stream format");
        }
        let mut sample_format_mask = 0u32;
        if pcm && pcm_size_rates & HDA_PCM_BITS_16 != 0 {
            sample_format_mask |= AudioSampleFormat::S16.mask();
        }
        if pcm && pcm_size_rates & HDA_PCM_BITS_24 != 0 {
            sample_format_mask |= AudioSampleFormat::S24In32.mask();
        }
        if pcm && pcm_size_rates & HDA_PCM_BITS_32 != 0 {
            sample_format_mask |= AudioSampleFormat::S32.mask();
        }
        if float32 && pcm_size_rates & HDA_PCM_BITS_32 != 0 {
            sample_format_mask |= AudioSampleFormat::F32.mask();
        }
        let caps = Self {
            rate_mask: (pcm_size_rates as u16) & HDA_PCM_RATE_BITS,
            sample_format_mask,
            min_channels: 1,
            max_channels: max_channels.min(MAX_PCM_CHANNELS as u8),
            interleaved: true,
            planar: false,
            hda_pcm: pcm,
            hda_float32: float32,
        };
        caps.validate()?;
        Ok(caps)
    }

    pub fn from_endpoint(endpoint: AudioEndpointInfo) -> Result<Self, &'static str> {
        let mut rate_mask = 0u16;
        for rate in HDA_RATES {
            if rate.rate_hz >= endpoint.min_rate_hz && rate.rate_hz <= endpoint.max_rate_hz {
                rate_mask |= 1u16 << rate.capability_bit;
            }
        }
        let caps = Self {
            rate_mask,
            sample_format_mask: endpoint.format_mask,
            min_channels: endpoint.min_channels.min(MAX_PCM_CHANNELS as u16) as u8,
            max_channels: endpoint.max_channels.min(MAX_PCM_CHANNELS as u16) as u8,
            interleaved: true,
            planar: false,
            hda_pcm: true,
            hda_float32: endpoint.format_mask & AudioSampleFormat::F32.mask() != 0,
        };
        caps.validate()?;
        Ok(caps)
    }

    #[must_use]
    pub fn supports_rate(&self, rate_hz: u32) -> bool {
        HDA_RATES.iter().any(|rate| {
            rate.rate_hz == rate_hz && self.rate_mask & (1u16 << rate.capability_bit) != 0
        })
    }

    #[must_use]
    pub fn supports_format(&self, format: AudioSampleFormat) -> bool {
        self.sample_format_mask & format.mask() != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PcmRequest {
    pub sample_format: AudioSampleFormat,
    pub requested_rate_hz: u32,
    pub channels: u8,
    pub storage: PcmStorageLayout,
    pub rate_policy: RatePolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PcmNegotiatedFormat {
    pub sample_format: AudioSampleFormat,
    pub rate_hz: u32,
    pub channel_map: ChannelMap,
    pub storage: PcmStorageLayout,
    pub frame_stride_bytes: u32,
    pub hda_stream_format: Option<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PcmPeriodGeometry {
    pub frame_stride_bytes: u32,
    pub period_frames: u32,
    pub period_bytes: u32,
    pub period_count: u32,
    pub ring_frames: u32,
    pub ring_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodedHdaFormat {
    pub rate_hz: u32,
    pub valid_bits: u8,
    pub channels: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct PcmQualificationReport {
    pub version: u32,
    pub canonical_formats: u8,
    pub canonical_rates: u8,
    pub hda_rate_roundtrips: u8,
    pub interleaved_planar: bool,
    pub channel_mapping: bool,
    pub exact_negotiation: bool,
    pub nearest_negotiation: bool,
    pub unsupported_rejected: bool,
    pub period_geometry: bool,
    pub hda_endpoint_bound: bool,
    pub hardware_rate_hz: u32,
    pub hardware_channels: u8,
    pub hardware_sample_format: AudioSampleFormat,
    pub fake_device: bool,
}

pub fn negotiate(
    capabilities: PcmCapabilities,
    request: PcmRequest,
) -> Result<PcmNegotiatedFormat, &'static str> {
    capabilities.validate()?;
    if request.channels < capabilities.min_channels || request.channels > capabilities.max_channels {
        return Err("requested PCM channel count is unsupported");
    }
    if !capabilities.supports_format(request.sample_format) {
        return Err("requested PCM sample format is unsupported");
    }
    match request.storage {
        PcmStorageLayout::Interleaved if !capabilities.interleaved => {
            return Err("requested interleaved PCM storage is unsupported");
        }
        PcmStorageLayout::Planar if !capabilities.planar => {
            return Err("requested planar PCM storage is unsupported");
        }
        _ => {}
    }
    let rate_hz = match request.rate_policy {
        RatePolicy::Exact => {
            if !capabilities.supports_rate(request.requested_rate_hz) {
                return Err("requested exact PCM sample rate is unsupported");
            }
            request.requested_rate_hz
        }
        RatePolicy::Nearest => nearest_supported_rate(capabilities.rate_mask, request.requested_rate_hz)
            .ok_or("PCM capability set has no negotiable rate")?,
    };
    let channel_map = ChannelMap::canonical(request.channels)?;
    channel_map.validate()?;
    let frame_stride_bytes = request
        .sample_format
        .bytes_per_sample()
        .checked_mul(u32::from(request.channels))
        .ok_or("PCM frame stride overflow")?;
    let hda_stream_format = if capabilities.hda_pcm && request.sample_format != AudioSampleFormat::F32 {
        Some(encode_hda_pcm_stream_format(
            rate_hz,
            sample_valid_bits(request.sample_format),
            request.channels,
        )?)
    } else {
        None
    };
    Ok(PcmNegotiatedFormat {
        sample_format: request.sample_format,
        rate_hz,
        channel_map,
        storage: request.storage,
        frame_stride_bytes,
        hda_stream_format,
    })
}

#[must_use]
pub fn nearest_supported_rate(rate_mask: u16, requested_hz: u32) -> Option<u32> {
    let mut best: Option<(u32, u32)> = None;
    for rate in HDA_RATES {
        if rate_mask & (1u16 << rate.capability_bit) == 0 {
            continue;
        }
        let distance = rate.rate_hz.abs_diff(requested_hz);
        match best {
            None => best = Some((rate.rate_hz, distance)),
            Some((best_rate, best_distance))
                if distance < best_distance || (distance == best_distance && rate.rate_hz < best_rate) =>
            {
                best = Some((rate.rate_hz, distance));
            }
            _ => {}
        }
    }
    best.map(|(rate, _)| rate)
}

pub fn encode_hda_pcm_stream_format(
    rate_hz: u32,
    valid_bits: u8,
    channels: u8,
) -> Result<u16, &'static str> {
    if channels == 0 || channels > 16 {
        return Err("HDA PCM stream channel count is outside 1..16");
    }
    let rate = HDA_RATES
        .iter()
        .find(|rate| rate.rate_hz == rate_hz)
        .ok_or("PCM sample rate has no HDA stream-format encoding")?;
    if rate.multiplier == 0 || rate.multiplier > 8 || rate.divisor == 0 || rate.divisor > 8 {
        return Err("HDA sample-rate multiplier/divisor is invalid");
    }
    let bits_code = match valid_bits {
        8 => 0u16,
        16 => 1u16,
        20 => 2u16,
        24 => 3u16,
        32 => 4u16,
        _ => return Err("PCM valid-bit width has no HDA encoding"),
    };
    let base = if rate.base_44k { 1u16 << 14 } else { 0 };
    let multiplier = u16::from(rate.multiplier - 1) << 11;
    let divisor = u16::from(rate.divisor - 1) << 8;
    Ok(base | multiplier | divisor | (bits_code << 4) | u16::from(channels - 1))
}

pub fn decode_hda_pcm_stream_format(value: u16) -> Result<DecodedHdaFormat, &'static str> {
    if value & (1 << 15) != 0 {
        return Err("HDA stream format is non-PCM");
    }
    let channels = ((value & 0x0f) + 1) as u8;
    let valid_bits = match (value >> 4) & 0x07 {
        0 => 8,
        1 => 16,
        2 => 20,
        3 => 24,
        4 => 32,
        _ => return Err("HDA PCM stream has reserved sample-size encoding"),
    };
    let base_hz = if value & (1 << 14) != 0 { 44_100u32 } else { 48_000u32 };
    let multiplier = u32::from(((value >> 11) & 0x07) + 1);
    let divisor = u32::from(((value >> 8) & 0x07) + 1);
    let numerator = base_hz
        .checked_mul(multiplier)
        .ok_or("HDA PCM rate decode overflow")?;
    if numerator % divisor != 0 {
        return Err("HDA PCM stream rate is fractional/unsupported");
    }
    let rate_hz = numerator / divisor;
    if !HDA_RATES.iter().any(|entry| entry.rate_hz == rate_hz) {
        return Err("HDA PCM stream rate is outside canonical rate table");
    }
    Ok(DecodedHdaFormat { rate_hz, valid_bits, channels })
}

pub fn period_geometry(
    format: PcmNegotiatedFormat,
    period_frames: u32,
    period_count: usize,
) -> Result<PcmPeriodGeometry, &'static str> {
    if period_frames == 0 {
        return Err("PCM period must contain at least one frame");
    }
    if period_count < 2 || period_count > MAX_AUDIO_DMA_PERIODS {
        return Err("PCM period count is outside K15.3 DMA bounds");
    }
    let period_bytes = format
        .frame_stride_bytes
        .checked_mul(period_frames)
        .ok_or("PCM period byte size overflow")?;
    let ring_frames = period_frames
        .checked_mul(period_count as u32)
        .ok_or("PCM ring frame count overflow")?;
    let ring_bytes = u64::from(period_bytes)
        .checked_mul(period_count as u64)
        .ok_or("PCM ring byte size overflow")?;
    if ring_bytes == 0 || ring_bytes > MAX_AUDIO_DMA_RING_BYTES {
        return Err("PCM ring geometry exceeds K15.3 DMA ring limit");
    }
    Ok(PcmPeriodGeometry {
        frame_stride_bytes: format.frame_stride_bytes,
        period_frames,
        period_bytes,
        period_count: period_count as u32,
        ring_frames,
        ring_bytes,
    })
}

pub fn convert_storage_layout(
    src: &[u8],
    dst: &mut [u8],
    sample_format: AudioSampleFormat,
    frames: usize,
    channels: usize,
    src_layout: PcmStorageLayout,
    dst_layout: PcmStorageLayout,
) -> Result<usize, &'static str> {
    validate_transform_geometry(sample_format, frames, channels, src, dst)?;
    let sample_bytes = sample_format.bytes_per_sample() as usize;
    let total = frames * channels * sample_bytes;
    if src_layout == dst_layout {
        dst[..total].copy_from_slice(&src[..total]);
        return Ok(total);
    }
    for frame in 0..frames {
        for channel in 0..channels {
            let src_sample = sample_index(frame, channel, frames, channels, src_layout) * sample_bytes;
            let dst_sample = sample_index(frame, channel, frames, channels, dst_layout) * sample_bytes;
            dst[dst_sample..dst_sample + sample_bytes]
                .copy_from_slice(&src[src_sample..src_sample + sample_bytes]);
        }
    }
    Ok(total)
}

pub fn remap_channels(
    src: &[u8],
    dst: &mut [u8],
    sample_format: AudioSampleFormat,
    frames: usize,
    src_map: ChannelMap,
    dst_map: ChannelMap,
    src_layout: PcmStorageLayout,
    dst_layout: PcmStorageLayout,
) -> Result<usize, &'static str> {
    src_map.validate()?;
    dst_map.validate()?;
    let src_channels = usize::from(src_map.channels);
    let dst_channels = usize::from(dst_map.channels);
    if frames == 0 || frames > MAX_PCM_CONVERSION_FRAMES {
        return Err("PCM channel-map frame count is outside bounded transform limit");
    }
    let sample_bytes = sample_format.bytes_per_sample() as usize;
    let src_bytes = frames
        .checked_mul(src_channels)
        .and_then(|v| v.checked_mul(sample_bytes))
        .ok_or("PCM channel-map source byte count overflow")?;
    let dst_bytes = frames
        .checked_mul(dst_channels)
        .and_then(|v| v.checked_mul(sample_bytes))
        .ok_or("PCM channel-map destination byte count overflow")?;
    if src.len() < src_bytes || dst.len() < dst_bytes {
        return Err("PCM channel-map buffer is smaller than requested geometry");
    }
    dst[..dst_bytes].fill(0);
    for dst_channel in 0..dst_channels {
        let position = dst_map.positions[dst_channel];
        let Some(src_channel) = src_map.index_of(position) else { continue; };
        for frame in 0..frames {
            let src_offset = sample_index(frame, src_channel, frames, src_channels, src_layout) * sample_bytes;
            let dst_offset = sample_index(frame, dst_channel, frames, dst_channels, dst_layout) * sample_bytes;
            dst[dst_offset..dst_offset + sample_bytes]
                .copy_from_slice(&src[src_offset..src_offset + sample_bytes]);
        }
    }
    Ok(dst_bytes)
}

fn validate_transform_geometry(
    sample_format: AudioSampleFormat,
    frames: usize,
    channels: usize,
    src: &[u8],
    dst: &[u8],
) -> Result<(), &'static str> {
    if frames == 0 || frames > MAX_PCM_CONVERSION_FRAMES {
        return Err("PCM transform frame count is outside bounded limit");
    }
    if channels == 0 || channels > MAX_PCM_CHANNELS {
        return Err("PCM transform channel count is outside bounded limit");
    }
    let bytes = frames
        .checked_mul(channels)
        .and_then(|v| v.checked_mul(sample_format.bytes_per_sample() as usize))
        .ok_or("PCM transform byte count overflow")?;
    if src.len() < bytes || dst.len() < bytes {
        return Err("PCM transform buffer is smaller than requested geometry");
    }
    Ok(())
}

#[inline]
fn sample_index(
    frame: usize,
    channel: usize,
    frames: usize,
    channels: usize,
    layout: PcmStorageLayout,
) -> usize {
    match layout {
        PcmStorageLayout::Interleaved => frame * channels + channel,
        PcmStorageLayout::Planar => channel * frames + frame,
    }
}

#[must_use]
pub const fn sample_valid_bits(format: AudioSampleFormat) -> u8 {
    match format {
        AudioSampleFormat::S16 => 16,
        AudioSampleFormat::S24In32 => 24,
        AudioSampleFormat::S32 | AudioSampleFormat::F32 => 32,
    }
}

fn find_hda_endpoint(direction: AudioDirection) -> Option<AudioEndpointInfo> {
    for device_index in 0..forgeaudio::device_count() {
        let device = forgeaudio::enumerate_device(device_index)?;
        if device.backend_kind != AUDIO_BACKEND_HDA {
            continue;
        }
        for endpoint_index in 0..device.endpoint_count as usize {
            let endpoint = forgeaudio::enumerate_endpoint(device.object_id, endpoint_index)?;
            if endpoint.direction == direction as u32 || endpoint.direction == AudioDirection::Duplex as u32 {
                return Some(endpoint);
            }
        }
    }
    None
}

pub fn run_self_test() -> Result<PcmQualificationReport, &'static str> {
    // 1. Every canonical HDA rate must encode and decode exactly.
    let mut rate_roundtrips = 0u8;
    for rate in HDA_RATES {
        let encoded = encode_hda_pcm_stream_format(rate.rate_hz, 16, 2)?;
        let decoded = decode_hda_pcm_stream_format(encoded)?;
        if decoded.rate_hz != rate.rate_hz || decoded.valid_bits != 16 || decoded.channels != 2 {
            return Err("HDA PCM rate/format roundtrip mismatch");
        }
        rate_roundtrips = rate_roundtrips.saturating_add(1);
    }

    // 2. Layout conversion is byte-exact for all four ForgeAudio canonical
    // sample/container formats. No arithmetic conversion is hidden here.
    for format in [
        AudioSampleFormat::S16,
        AudioSampleFormat::S24In32,
        AudioSampleFormat::S32,
        AudioSampleFormat::F32,
    ] {
        let sample_bytes = format.bytes_per_sample() as usize;
        let frames = 7usize;
        let channels = 3usize;
        let total = frames * channels * sample_bytes;
        let mut interleaved = [0u8; 256];
        let mut planar = [0u8; 256];
        let mut roundtrip = [0u8; 256];
        for (index, byte) in interleaved[..total].iter_mut().enumerate() {
            *byte = ((index * 37 + sample_bytes * 11) & 0xff) as u8;
        }
        convert_storage_layout(
            &interleaved[..total], &mut planar[..total], format, frames, channels,
            PcmStorageLayout::Interleaved, PcmStorageLayout::Planar,
        )?;
        convert_storage_layout(
            &planar[..total], &mut roundtrip[..total], format, frames, channels,
            PcmStorageLayout::Planar, PcmStorageLayout::Interleaved,
        )?;
        if roundtrip[..total] != interleaved[..total] {
            return Err("PCM interleaved/planar roundtrip changed sample bytes");
        }
    }

    // 3. Channel mapping copies like-named channels and zero-fills channels
    // that do not exist in the source. This is mapping, not mixing/upmix DSP.
    let stereo = ChannelMap::canonical(2)?;
    let surround = ChannelMap::canonical(6)?;
    let frames = 4usize;
    let mut stereo_bytes = [0u8; 32];
    for frame in 0..frames {
        let left = (0x1000u16 + frame as u16).to_le_bytes();
        let right = (0x2000u16 + frame as u16).to_le_bytes();
        stereo_bytes[frame * 4..frame * 4 + 2].copy_from_slice(&left);
        stereo_bytes[frame * 4 + 2..frame * 4 + 4].copy_from_slice(&right);
    }
    let mut surround_bytes = [0xa5u8; 64];
    let mapped_bytes = remap_channels(
        &stereo_bytes[..frames * 4], &mut surround_bytes,
        AudioSampleFormat::S16, frames, stereo, surround,
        PcmStorageLayout::Interleaved, PcmStorageLayout::Interleaved,
    )?;
    if mapped_bytes != frames * 12 {
        return Err("PCM channel-map destination byte count mismatch");
    }
    for frame in 0..frames {
        let base = frame * 12;
        if surround_bytes[base..base + 4] != stereo_bytes[frame * 4..frame * 4 + 4] {
            return Err("PCM channel-map failed to preserve front-left/front-right");
        }
        if surround_bytes[base + 4..base + 12].iter().any(|&byte| byte != 0) {
            return Err("PCM channel-map failed to zero-fill absent surround channels");
        }
    }

    // 4. Capability parsing + exact and deterministic nearest-rate negotiation.
    let vector_caps = PcmCapabilities::from_hda_parameters(
        u32::from((1u16 << 5) | (1u16 << 6) | (1u16 << 8) | (1u16 << 10))
            | HDA_PCM_BITS_16 | HDA_PCM_BITS_24 | HDA_PCM_BITS_32,
        HDA_STREAM_FORMAT_PCM | HDA_STREAM_FORMAT_FLOAT32,
        8,
    )?;
    let exact = negotiate(vector_caps, PcmRequest {
        sample_format: AudioSampleFormat::S24In32,
        requested_rate_hz: 96_000,
        channels: 6,
        storage: PcmStorageLayout::Interleaved,
        rate_policy: RatePolicy::Exact,
    })?;
    if exact.rate_hz != 96_000 || exact.frame_stride_bytes != 24 {
        return Err("PCM exact negotiation returned wrong format geometry");
    }
    let exact_hda = exact.hda_stream_format.ok_or("PCM exact HDA negotiation lacked stream format")?;
    let exact_decoded = decode_hda_pcm_stream_format(exact_hda)?;
    if exact_decoded.rate_hz != 96_000 || exact_decoded.valid_bits != 24 || exact_decoded.channels != 6 {
        return Err("PCM exact negotiation HDA encoding mismatch");
    }
    let nearest = negotiate(vector_caps, PcmRequest {
        sample_format: AudioSampleFormat::S16,
        requested_rate_hz: 50_000,
        channels: 2,
        storage: PcmStorageLayout::Interleaved,
        rate_policy: RatePolicy::Nearest,
    })?;
    if nearest.rate_hz != 48_000 {
        return Err("PCM nearest-rate negotiation did not choose 48 kHz");
    }
    // The synthetic vector advertises rate bits 5, 6, 8 and 10 only:
    // 44.1, 48, 96 and 192 kHz. 176.4 kHz (bit 9) is deliberately absent.
    // Prove the negative-test premise before asking negotiation to reject it.
    if vector_caps.supports_rate(176_400) || vector_caps.max_channels >= 9 {
        return Err("PCM negative-test capability vector unexpectedly supports rejected request");
    }
    let unsupported_rejected = negotiate(vector_caps, PcmRequest {
        sample_format: AudioSampleFormat::S16,
        requested_rate_hz: 176_400,
        channels: 2,
        storage: PcmStorageLayout::Interleaved,
        rate_policy: RatePolicy::Exact,
    }).is_err()
        && negotiate(vector_caps, PcmRequest {
            sample_format: AudioSampleFormat::S16,
            requested_rate_hz: 48_000,
            channels: 9,
            storage: PcmStorageLayout::Interleaved,
            rate_policy: RatePolicy::Exact,
        }).is_err();
    if !unsupported_rejected {
        return Err("PCM unsupported exact rate/channel request was not rejected");
    }
    let geometry = period_geometry(exact, 256, 4)?;
    if geometry.period_bytes != 6_144 || geometry.ring_bytes != 24_576 || geometry.ring_frames != 1_024 {
        return Err("PCM period/ring geometry calculation mismatch");
    }

    // 5. Bind the engine to the real HDA endpoints registered by frozen K15.4.
    let playback = find_hda_endpoint(AudioDirection::Playback)
        .ok_or("K15.5 could not find K15.4 HDA playback endpoint")?;
    let capture = find_hda_endpoint(AudioDirection::Capture)
        .ok_or("K15.5 could not find K15.4 HDA capture endpoint")?;
    let playback_caps = PcmCapabilities::from_endpoint(playback)?;
    let capture_caps = PcmCapabilities::from_endpoint(capture)?;
    let hardware_playback = negotiate(playback_caps, PcmRequest {
        sample_format: AudioSampleFormat::S16,
        requested_rate_hz: 48_000,
        channels: 2,
        storage: PcmStorageLayout::Interleaved,
        rate_policy: RatePolicy::Exact,
    })?;
    let hardware_capture = negotiate(capture_caps, PcmRequest {
        sample_format: AudioSampleFormat::S16,
        requested_rate_hz: 48_000,
        channels: 2,
        storage: PcmStorageLayout::Interleaved,
        rate_policy: RatePolicy::Exact,
    })?;
    if hardware_playback.hda_stream_format != Some(0x0011)
        || hardware_capture.hda_stream_format != Some(0x0011)
    {
        return Err("K15.5 real HDA endpoint did not negotiate frozen 48k/S16/stereo format");
    }

    // 6. Existing ABI stream configuration must agree with the negotiated
    // real endpoint geometry without changing/forking frozen ABI v1.
    let stream_config = AudioStreamConfig {
        abi_version: FORGEAUDIO_ABI_VERSION,
        flags: 0,
        direction: AudioDirection::Playback as u32,
        sample_format: hardware_playback.sample_format as u32,
        sample_rate_hz: hardware_playback.rate_hz,
        channels: u16::from(hardware_playback.channel_map.channels),
        reserved0: 0,
        period_frames: 256,
        buffer_frames: 1_024,
        reserved1: 0,
    };
    stream_config.validate()?;
    if stream_config.frame_stride_bytes() != Some(hardware_playback.frame_stride_bytes) {
        return Err("K15.5 negotiated PCM frame stride disagrees with frozen ABI v1");
    }

    serial::println(format_args!(
        "[K15PCM] canonical engine: version={} formats=4 rates={} max_channels={} interleaved=true planar=true allocation_free=true bounded_frames={}",
        FORGEAUDIO_PCM_ENGINE_VERSION, HDA_RATES.len(), MAX_PCM_CHANNELS, MAX_PCM_CONVERSION_FRAMES
    ));
    serial::println(format_args!(
        "[K15PCM] HDA format engine: rate_roundtrips={} exact=96000/S24in32/6ch nearest_50000=48000 stream_format={:#06x} unsupported_rejected=true",
        rate_roundtrips, exact_hda
    ));
    serial::println(format_args!(
        "[K15PCM] layout+channel engine: interleaved_planar=true formats=4 channel_mapping=true zero_fill=true no_mixing=true"
    ));
    serial::println(format_args!(
        "[K15PCM] DMA geometry: period_frames={} period_bytes={} periods={} ring_frames={} ring_bytes={} within_k15_3=true",
        geometry.period_frames, geometry.period_bytes, geometry.period_count, geometry.ring_frames, geometry.ring_bytes
    ));
    serial::println(format_args!(
        "[K15PCM] real HDA endpoint binding: playback=true capture=true rate={} channels={} format=S16 hda_stream=0x0011 fake_device=false",
        hardware_playback.rate_hz, hardware_playback.channel_map.channels
    ));
    serial::println(format_args!(
        "[K15OK] K15.5 ForgeAudio PCM format engine qualified: canonical=true interleaved_planar=true channel_mapping=true rate_negotiation=true HDA_encode_decode=true period_geometry=true HDA_endpoint=true unsupported_rejected=true fake_device=false"
    ));

    Ok(PcmQualificationReport {
        version: FORGEAUDIO_PCM_ENGINE_VERSION,
        canonical_formats: 4,
        canonical_rates: HDA_RATES.len() as u8,
        hda_rate_roundtrips: rate_roundtrips,
        interleaved_planar: true,
        channel_mapping: true,
        exact_negotiation: true,
        nearest_negotiation: true,
        unsupported_rejected,
        period_geometry: true,
        hda_endpoint_bound: true,
        hardware_rate_hz: hardware_playback.rate_hz,
        hardware_channels: hardware_playback.channel_map.channels,
        hardware_sample_format: hardware_playback.sample_format,
        fake_device: false,
    })
}
