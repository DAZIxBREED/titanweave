#!/usr/bin/env python3
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
pcm = (root / 'kernel/weavecore/src/forgeaudio_pcm.rs').read_text()
main = (root / 'kernel/weavecore/src/main.rs').read_text()
stone = (root / 'K15_STONE_CONTRACT.md').read_text()
status = (root / 'K15_5_SOURCE_STATUS.md').read_text()
implementation = (root / 'K15_5_IMPLEMENTATION.md').read_text()
runner = (root / 'tools/run-k15-5-qemu-forgeaudio-pcm.sh').read_text()
checker = (root / 'tools/check-k15-5-serial-log.sh').read_text()
k154 = (root / 'K15_4_SOURCE_STATUS.md').read_text()

assert 'Status: **QUALIFIED / FROZEN**' in k154
assert '5. **K15.5 — PCM Format Engine**' in stone
assert 'Status: **SOURCE-INTEGRATED / RUNTIME QUALIFICATION PENDING**' in status

for token in [
    'FORGEAUDIO_PCM_ENGINE_VERSION: u32 = 1',
    'MAX_PCM_CHANNELS: usize = 16',
    'MAX_PCM_CONVERSION_FRAMES: usize = 2048',
    'PcmStorageLayout', 'Interleaved', 'Planar',
    'ChannelPosition', 'ChannelMap', 'canonical(channels: u8)',
    'PcmCapabilities', 'from_hda_parameters', 'from_endpoint',
    'RatePolicy', 'Exact', 'Nearest', 'nearest_supported_rate',
    'PcmRequest', 'PcmNegotiatedFormat', 'PcmPeriodGeometry',
    'encode_hda_pcm_stream_format', 'decode_hda_pcm_stream_format',
    'period_geometry', 'convert_storage_layout', 'remap_channels',
    'MAX_AUDIO_DMA_PERIODS', 'MAX_AUDIO_DMA_RING_BYTES',
    'AudioSampleFormat::S16', 'AudioSampleFormat::S24In32',
    'AudioSampleFormat::S32', 'AudioSampleFormat::F32',
    'AUDIO_BACKEND_HDA', 'find_hda_endpoint',
    'hardware_playback.hda_stream_format != Some(0x0011)',
    '[K15PCM] canonical engine:', '[K15PCM] HDA format engine:',
    '[K15PCM] layout+channel engine:', '[K15PCM] DMA geometry:',
    '[K15PCM] real HDA endpoint binding:',
    '[K15OK] K15.5 ForgeAudio PCM format engine qualified:',
]:
    assert token in pcm, token

# Frozen K15.5 scope must remain format-only: no server/graph/resampler/mixer.
for forbidden in ['struct ForgeAudioD', 'Resampler', 'MixerNode', 'GraphEngine']:
    assert forbidden not in pcm, forbidden
for forbidden in ['todo!()', 'unimplemented!()', 'panic!("TODO', '.unwrap()']:
    assert forbidden not in pcm, forbidden

assert 'mod forgeaudio_pcm;' in main
assert 'forgeaudio_pcm::run_self_test()' in main
assert '[K15PR] ForgeAudio PCM ready:' in main
assert main.index('forgeaudio_hda::initialize_and_qualify') < main.index('forgeaudio_pcm::run_self_test()')

# Parse and verify the complete canonical HDA rate table.
rate_pattern = re.compile(
    r'HdaRateSpec \{ rate_hz: ([0-9_]+), capability_bit: (\d+), base_44k: (true|false), multiplier: (\d+), divisor: (\d+) \}'
)
rates = []
for m in rate_pattern.finditer(pcm):
    rates.append((int(m.group(1).replace('_','')), int(m.group(2)), m.group(3) == 'true', int(m.group(4)), int(m.group(5))))
expected = [
    (8000,0,False,1,6), (11025,1,True,1,4), (16000,2,False,1,3),
    (22050,3,True,1,2), (32000,4,False,2,3), (44100,5,True,1,1),
    (48000,6,False,1,1), (88200,7,True,2,1), (96000,8,False,2,1),
    (176400,9,True,4,1), (192000,10,False,4,1), (384000,11,False,8,1),
]
assert rates[:12] == expected, rates[:12]

# Independently validate the HDA 16-bit PCM stream encoding vectors used by
# the Rust engine. Base, multiplier, divisor and channels-1 are all bounded.
def encode(rate, bits=16, channels=2):
    spec = next(r for r in expected if r[0] == rate)
    _, _, base44, mult, div = spec
    bits_code = {8:0,16:1,20:2,24:3,32:4}[bits]
    return ((1<<14) if base44 else 0) | ((mult-1)<<11) | ((div-1)<<8) | (bits_code<<4) | (channels-1)

assert encode(48000, 16, 2) == 0x0011
assert encode(44100, 16, 2) == 0x4011
assert encode(96000, 24, 6) == 0x0835
assert encode(192000, 32, 8) == 0x1847

# Bounded RT-safe transforms: explicit cap + no heap/Vec/Box in the engine.
assert 'frames > MAX_PCM_CONVERSION_FRAMES' in pcm
assert 'channels > MAX_PCM_CHANNELS' in pcm
for forbidden in ['Vec<', 'Box<', 'alloc::', 'std::']:
    assert forbidden not in pcm, forbidden

# Negotiation must fail closed on exact unsupported values and must not perform
# hidden mixing/resampling.
assert 'requested exact PCM sample rate is unsupported' in pcm
assert 'requested PCM channel count is unsupported' in pcm
assert 'requested PCM sample format is unsupported' in pcm
assert 'nearest.rate_hz != 48_000' in pcm
assert 'requested_rate_hz: 176_400' in pcm
assert 'vector_caps.supports_rate(176_400)' in pcm
assert 'requested_rate_hz: 192_000' not in pcm[pcm.index('let unsupported_rejected'):pcm.index('let geometry = period_geometry')]
assert main.index('[K15CO] HDA/GPU coexistence:') < main.index('forgeaudio_pcm::run_self_test()')
assert 'TITANWEAVE_ALLOW_LATER_GATE_FAILURES=1' in checker
assert 'zero-fill' in pcm or 'zero_fill' in pcm
assert 'no_mixing=true' in pcm

# K15.3 ring bounds are consumed rather than redefined with wider authority.
assert 'period_count < 2 || period_count > MAX_AUDIO_DMA_PERIODS' in pcm
assert 'ring_bytes > MAX_AUDIO_DMA_RING_BYTES' in pcm

for token in [
    'K15.5 ForgeAudio PCM format engine', 'ich9-intel-hda', 'hda-duplex',
    'check-k15-5-serial-log.sh',
]:
    assert token in runner, token
for token in [
    '[K15OK] K15.4 ForgeAudio real HDA hardware backend qualified:',
    '[K15PCM] canonical engine:', '[K15PCM] HDA format engine:',
    '[K15PCM] layout+channel engine:', '[K15PCM] DMA geometry:',
    '[K15PCM] real HDA endpoint binding:',
    '[K15OK] K15.5 ForgeAudio PCM format engine qualified:',
    '[K15PR] ForgeAudio PCM ready:',
]:
    assert token in checker, token

assert 'interleaved' in implementation.lower()
assert 'planar' in implementation.lower()
assert 'channel' in implementation.lower()
assert 'rate' in implementation.lower()
assert '0x0011' in implementation

print('Titanweave K15.5 ForgeAudio PCM format engine source checks passed.')
