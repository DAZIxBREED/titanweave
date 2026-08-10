#!/usr/bin/env python3
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
dma = (root / 'kernel/weavecore/src/forgeaudio_dma.rs').read_text()
main = (root / 'kernel/weavecore/src/main.rs').read_text()
paging = (root / 'kernel/weavecore/src/paging.rs').read_text()
translated = (root / 'kernel/weavecore/src/translated_dma.rs').read_text()
stone = (root / 'K15_STONE_CONTRACT.md').read_text()
implementation = (root / 'K15_3_IMPLEMENTATION.md').read_text()

for token in [
    'FORGEAUDIO_DMA_TRANSPORT_VERSION: u32 = 1',
    'MAX_AUDIO_DMA_PERIODS: usize = 32',
    'MAX_AUDIO_DMA_RING_BYTES',
    'PeriodOwnership',
    'CpuWritable',
    'QueuedToDevice',
    'DeviceReady',
    'DeviceOwned',
    'CpuReadable',
    'DmaIsolationLease',
    'new_translated',
    'hardware_translated: true',
    'AudioDmaBuffer',
    'paging::map_kernel_dma',
    'paging::unmap_kernel_dma',
    'allocate_contiguous',
    'deallocate_contiguous',
    'AudioDmaTransport',
    'queue_playback_period',
    'release_capture_period',
    'arm_hardware',
    'backend_acquire_next',
    'backend_complete_period',
    'frame_position',
    'wrap_count',
    'underruns',
    'overruns',
    'run_self_test',
    '[K15OK] K15.3 ForgeAudio audio DMA transport qualified:',
]:
    assert token in dma, token

# Production backend entry points must be fail-closed without a translated lease.
assert 'if !self.hardware_armed' in dma
assert 'audio DMA backend cannot acquire an unarmed transport' in dma
assert 'audio DMA backend cannot complete an unarmed transport' in dma
assert 'audio DMA hardware arm requires translated IOMMU isolation' in dma
assert 'lease.validate_for(&self.ring, self.direction)?' in dma

# The self-test must not create or register an audio device and must explicitly
# separate contract accounting from hardware audio completion.
selftest = dma.split('pub fn run_self_test(', 1)[1]
assert 'register_device(' not in selftest
assert 'register_endpoint(' not in selftest
assert 'hardware_audio=false fake_dma=false' in selftest
assert 'completion_source=transport_core_selftest' in selftest
assert 'raw_arm_rejected' in selftest
assert 'playback underrun was not detected' in selftest
assert 'capture overrun was not detected' in selftest

for token in ['KERNEL_DMA_BASE', 'map_kernel_dma', 'unmap_kernel_dma']:
    assert token in paging, token
for token in ['hardware_translation_qualified', 'TranslationQualification', 'QEMU_EDU_DEVICE']:
    assert token in translated, token

assert 'mod forgeaudio_dma;' in main
assert 'forgeaudio_dma::run_self_test(' in main
assert '[K15DR] ForgeAudio DMA ready:' in main
# K15.3 must run only after the already-qualified K14.B translated-DMA proof.
assert main.index('translated_dma::initialize_qualification(') < main.index('forgeaudio_dma::run_self_test(')

# Stone contract remains exactly 16 gates and K15.3 wording is unchanged.
gates = re.findall(r'^\d+\. \*\*K15\.(\d+) — ', stone, flags=re.MULTILINE)
assert gates == [str(i) for i in range(1, 17)], gates
assert '**K15.3 — Audio DMA Transport**' in stone
assert 'Cyclic DMA, period completion, position tracking, IOMMU isolation, buffer ownership and XRUN detection.' in stone
assert 'K15 ends at K15.16.' in stone

for forbidden in ['todo!', 'unimplemented!']:
    assert forbidden not in dma.lower(), forbidden

for required in ['cyclic DMA', 'IOMMU', 'ownership', 'XRUN', 'no audio device']:
    assert required.lower() in implementation.lower(), required

print('Titanweave K15.3 ForgeAudio audio DMA transport source checks passed.')
