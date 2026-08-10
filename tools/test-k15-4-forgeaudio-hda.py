#!/usr/bin/env python3
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
hda = (root / 'kernel/weavecore/src/forgeaudio_hda.rs').read_text()
dma = (root / 'kernel/weavecore/src/forgeaudio_dma.rs').read_text()
translated = (root / 'kernel/weavecore/src/translated_dma.rs').read_text()
main = (root / 'kernel/weavecore/src/main.rs').read_text()
abi = (root / 'libraries/forgeaudio-abi/src/lib.rs').read_text()
runner = (root / 'tools/run-k15-4-qemu-forgeaudio-hda.sh').read_text()
checker = (root / 'tools/check-k15-4-serial-log.sh').read_text()
stone = (root / 'K15_STONE_CONTRACT.md').read_text()
implementation = (root / 'K15_4_IMPLEMENTATION.md').read_text()

for token in [
    'FORGEAUDIO_HDA_BACKEND_VERSION: u32 = 1',
    'HDA_CLASS_MULTIMEDIA: u8 = 0x04',
    'HDA_SUBCLASS_AUDIO: u8 = 0x03',
    'REG_GCAP', 'REG_GCTL', 'GCTL_CRST',
    'REG_CORBLBASE', 'REG_CORBWP', 'REG_CORBCTL',
    'REG_RIRBLBASE', 'REG_RIRBWP', 'REG_RIRBCTL',
    'CommandRings', 'discover_topology', 'configure_codec',
    'PARAM_AUDIO_WIDGET_CAPS', 'WIDGET_AUDIO_OUTPUT', 'WIDGET_AUDIO_INPUT',
    'BdlEntry', 'write_bdl_single', 'program_stream_static',
    'msi::enable_msi', 'hda_irq_handler', 'STREAM_IRQ_EVENTS',
    'AudioDmaTransport::allocate', 'DmaIsolationLease::new_translated',
    'backend_acquire_next', 'backend_complete_period', 'backend_abort_inflight',
    'translated_dma::with_temporary_translated_domain',
    'capture_period_changed', 'register_forgeaudio_device',
    '[K15OK] K15.4 ForgeAudio real HDA hardware backend qualified:',
]:
    assert token in hda, token

# K15.4 must discover and claim a real PCI HDA function rather than creating a
# synthetic device object.
assert 'pci::find_first(|f| f.class_code == HDA_CLASS_MULTIMEDIA && f.subclass == HDA_SUBCLASS_AUDIO)' in hda
assert 'forgebus::claim_pci_function(hda, b"forgeaudio-hda", 2)' in hda
assert 'paging::map_kernel_mmio' in hda
assert 'pci::enable_bus_master(hda)' not in hda  # target BM is owned by translated-domain helper

# Completion must depend on real interrupt accounting before the K15.3 period
# retirement API is called.
run_fn = hda.split('fn run_stream_one_period', 1)[1]
assert 'STREAM_IRQ_EVENTS.load' in run_fn
assert 'HDA stream did not produce an MSI completion' in run_fn
assert 'x86_64::interrupts_enabled()' in run_fn
assert 'x86_64::enable_interrupts()' in run_fn
assert 'x86_64::disable_interrupts()' in run_fn
assert 'INTCTL' in run_fn and 'INTSTS' in run_fn
assert 'IF_before' in run_fn and 'IF_after' in run_fn
hardware_section = hda.split('for _ in 0..TEST_PERIODS_PER_DIRECTION', 1)[1]
assert hardware_section.index('run_stream_one_period') < hardware_section.index('backend_complete_period')

# Data-DMA translation is mandatory and bounded to the exact requester.
for token in [
    'TemporaryDmaRegion', 'TemporaryTranslatedDomain',
    'with_temporary_translated_domain',
    'target_bus_master=true', 'target_bus_master=false',
    'tables.install_root_context(requester, domain_id)',
    'vtd.enable_translation()', 'pci::enable_bus_master(function)',
    'pci::disable_bus_master(function)', 'vtd.disable_translation()',
]:
    assert token in translated, token
assert translated.index('vtd.enable_translation()') < translated.index('pci::enable_bus_master(function)')
assert 'VTD_CONTEXT_TT_PASS_THROUGH: u64 = 2 << 2' in translated
assert 'VTD_ECAP_PASS_THROUGH: u64 = 1 << 6' in translated
assert 'install_pass_through_context' in translated
assert '[IOMP] temporary translated coexistence:' in translated
assert 'unrelated_busmasters_untouched=true' in translated
assert 'pci::disable_bus_master(candidate)' not in translated
assert 'peers_preserved=true' in translated

# Frozen K15.3 gets read-only geometry accessors plus a bounded recovery-only
# in-flight abort needed to guarantee K15.4 hardware teardown. Normal success
# still retires periods only through backend_complete_period.
for token in ['ring_physical_base', 'ring_mapped_bytes', 'period_bytes', 'period_frames', 'frame_stride_bytes']:
    assert token in dma, token
assert 'pub fn backend_abort_inflight' in dma
assert 'period.ownership = PeriodOwnership::CpuWritable' in dma
assert 'period.ownership = PeriodOwnership::DeviceReady' in dma
assert 'backend_abort_inflight' in hda
assert 'unwrap()' not in hda
assert 'write8(mmio + REG_CORBSIZE' not in hda
assert 'write8(mmio + REG_RIRBSIZE' not in hda
assert 'selected_ring_entries' in hda
assert 'ring_selection_is_advertised' in hda
assert 'CORBRP reset assert timed out' in hda
assert 'acknowledge_rirb_status' in hda
assert 'HDA CORB DMA fetch timed out before codec command execution' in hda
assert 'command timeout diagnostic:' in hda

# Temporary VT-d setup must reclaim table pages on mapping errors and must
# refuse to arm if its bounded PCI bus-master snapshot overflows.
assert 'let mapping_result = (|| -> Result<u32' in translated
assert 'Err(error) => { tables.release(allocator); return Err(error); }' in translated
assert 'snapshot_overflow' in translated
assert 'temporary translated DMA PCI snapshot capacity exceeded' in translated

assert 'AUDIO_BACKEND_HDA: u8 = 1' in abi
assert 'mod forgeaudio_hda;' in main
assert 'forgeaudio_hda::initialize_and_qualify(' in main
assert '[K15HR] ForgeAudio HDA ready:' in main
assert main.index('forgeaudio_dma::run_self_test(') < main.index('forgeaudio_hda::initialize_and_qualify(')
assert '[K15CO] HDA/GPU coexistence:' in main
assert 'virtio_gpu::suspend_presentation_for_recovery()' in main
assert 'virtio_gpu::resume_presentation_after_recovery()' in main
assert '[K6DIAG] user runtime failure detail:' in (root / 'kernel/weavecore/src/process.rs').read_text()

# QEMU must instantiate an HDA controller+duplex codec with MSI.  Data IOMMU
# stays active, but interrupt remapping is intentionally disabled because the
# current MSI primitive programs direct APIC MSI messages.
for token in [
    'ich9-intel-hda,id=twk15hda,msi=on',
    'hda-duplex,bus=twk15hda.0,cad=0,audiodev=twk15audio',
    'driver=none,id=twk15audio',
    'intel-iommu,intremap=off,caching-mode=on',
]:
    assert token in runner, token

for token in [
    '[K15HDA] controller:', '[IOMP] temporary translated coexistence:', '[IOMA] temporary translated device domain armed:',
    '[K15HDA] command+codec:', '[K15HDA] DMA+IRQ:',
    '[IOMV] temporary translated device domain revoked:',
    '[K15HDA] ForgeAudio registry:',
    '[K15OK] K15.4 ForgeAudio real HDA hardware backend qualified:',
    '[K15HR] ForgeAudio HDA ready:', '[K15CO] HDA/GPU coexistence:',
]:
    assert token in checker, token

# No fake/stub success is allowed in the new required path.
for forbidden in ['todo!', 'unimplemented!', 'fake_irq=true', 'fake_hw=true', 'placeholder device']:
    assert forbidden not in hda.lower(), forbidden
assert 'fake_hw=false' in hda
assert 'physical_silicon=false' in hda

# Stone contract remains exactly 16 gates; K15.5 remains separate.
gates = re.findall(r'^\d+\. \*\*K15\.(\d+) — ', stone, flags=re.MULTILINE)
assert gates == [str(i) for i in range(1, 17)], gates
assert '**K15.4 — Real HDA Hardware Backend**' in stone
assert 'PCI HDA controller, CORB/RIRB, codec/widget discovery, BDL/stream descriptors, interrupts, playback and capture.' in stone
assert '**K15.5 — PCM Format Engine**' in stone
assert 'K15 ends at K15.16.' in stone

for required in ['CORB/RIRB', 'BDL', 'MSI', 'playback', 'capture', 'translated', '48 kHz', 'K15.5']:
    assert required.lower() in implementation.lower(), required

# Preserve the repository-level project identity/status documents in milestone
# artifacts so rsync-based source promotion cannot erase them again.
for doc in ['README.md', 'PROJECT_VISION.md', 'CURRENT_STATUS.md']:
    assert (root / doc).is_file(), doc
assert 'K15.4 | Real HDA Hardware Backend' in (root / 'README.md').read_text()
assert 'K15.4 Real HDA Hardware Backend: QUALIFIED / FROZEN' in (root / 'CURRENT_STATUS.md').read_text()

print('Titanweave K15.4 ForgeAudio real HDA hardware backend source checks passed.')
