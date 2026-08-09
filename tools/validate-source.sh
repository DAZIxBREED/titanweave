#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
python3 - "$ROOT" <<'PY'
from pathlib import Path
import sys, tomllib
root=Path(sys.argv[1])
with (root/'Cargo.toml').open('rb') as f: cargo=tomllib.load(f)
assert cargo['workspace']['package']['version']=='0.14.0'
boot=(root/'libraries/boot-protocol/src/lib.rs').read_text()
for token in ['BOOT_PROTOCOL_VERSION: u32 = 14','BOOT_VOLUME','BootModulesInfo','TITANV14','FramebufferInfo']:
    assert token in boot,token
loader=(root/'boot/uefi-loader/src/main.rs').read_text()
for token in ['Titanweave UEFI loader K14','TITANFS.IMG','BOOT_VOLUME','load_boot_module','capture_framebuffer','GraphicsOutput']:
    assert token in loader,token
main=(root/'kernel/weavecore/src/main.rs').read_text()
for token in ['WeaveCore K14','archive_service::initialize','mount_boot_volume','storage::initialize','display::initialize','gpu_runtime::initialize_foundation']:
    assert token in main,token
checks={
 'kernel/weavecore/src/sha256.rs':['Sha256','constant_time_eq','digest'],
 'kernel/weavecore/src/capability.rs':['CapabilitySet','SubjectKind','maximum_for'],
 'kernel/weavecore/src/trust.rs':['TrustStore','SignedManifest','SignatureVerifier','revoke'],
 'kernel/weavecore/src/update.rs':['UpdateManager','anti-rollback','PendingBoot'],
 'kernel/weavecore/src/trust_service.rs':['K10 trust','SelfTestVerifier'],
 'kernel/weavecore/src/archive.rs':['ArchiveFormat','SevenZip','PackAndUnpack','validate_relative_path','MAX_EXPANSION_RATIO'],
 'kernel/weavecore/src/archive_service.rs':['ARCHIVE_PROTOCOL_VERSION','ArchiveQueue','InstallPackage','Vaultforge'],
 'kernel/weavecore/src/package.rs':['PackageTransaction','RollbackReady','MANIFEST_PATH','SIGNATURE_PATH','CHECKSUMS_PATH'],
 'kernel/weavecore/src/gpt.rs':['GPT_SIGNATURE','GptPartition'],
 'kernel/weavecore/src/automount.rs':['AutoMountReport','MountedVolume'],
 'kernel/weavecore/src/volume_events.rs':['VolumeEventKind','EventRing'],
 'kernel/weavecore/src/mount_namespace.rs':['MountNamespace','MountGrant'],
 'kernel/weavecore/src/vfs.rs':['mount_boot_volume','with_file'],
 'kernel/weavecore/src/service.rs':['ARCHIVE.ELF','DRIVERD.ELF','DISPLAYD.ELF','ServiceRole::Display'],
 'kernel/weavecore/src/device.rs':['DeviceRegistry','DeviceState','MAX_DEVICES'],
 'kernel/weavecore/src/driver.rs':['DriverRegistry','DriverIsolation','report_crash'],
 'kernel/weavecore/src/dma.rs':['DmaManager','DmaDomain','unmap'],
 'kernel/weavecore/src/iommu.rs':['IommuBackend','IommuPolicy'],
 'kernel/weavecore/src/interrupt_router.rs':['InterruptRouter','FIRST_DEVICE_VECTOR','SYSCALL_VECTOR'],
 'kernel/weavecore/src/hotplug.rs':['HotplugJournal','HotplugKind'],
 'kernel/weavecore/src/nvme.rs':['NvmeController','validate_lba'],
 'kernel/weavecore/src/usb_hid.rs':['BootKeyboardState','KeyEvent'],
 'kernel/weavecore/src/forgebus.rs':['ForgeBusReport','pci::enumerate'],
 'kernel/weavecore/src/framebuffer.rs':['Framebuffer','draw_boot_card','write_volatile'],
 'kernel/weavecore/src/graphics_abi.rs':['GRAPHICS_ABI_VERSION','DisplayInfo','SurfaceCreate','PresentRequest'],
 'kernel/weavecore/src/forgegraphics.rs':['FORGEGRAPHICS_ABI_VERSION','AdapterRegistry','CAP_MULTI_GPU_COPY'],
 'kernel/weavecore/src/compositor.rs':['SurfaceRegistry','DamageTracker','hit_test'],
 'kernel/weavecore/src/input_router.rs':['InputRouter','capture_pointer','route_key'],
 'kernel/weavecore/src/display.rs':['K13 GOP scanout online','packed_primary_mode'],
 'kernel/weavecore/src/workplace_shell.rs':['render_preview','WorkplacePreviewReport'],
 'kernel/weavecore/src/gpu_topology.rs':['GpuTopologyReport','VENDOR_AMD','VENDOR_INTEL','VENDOR_NVIDIA','VENDOR_VIRTIO'],
 'kernel/weavecore/src/gpu_memory.rs':['GpuMemoryManager','MemoryDomain','migrate','pin'],
 'kernel/weavecore/src/gpu_queue.rs':['CommandQueue','CommandPacket','GPU_QUEUE_DEPTH'],
 'kernel/weavecore/src/gpu_fence.rs':['TimelineFence','issue','complete'],
 'kernel/weavecore/src/gpu_modeset.rs':['AtomicModeRequest','DisplayMode'],
 'kernel/weavecore/src/gpu_multigpu.rs':['TransferRoute','PeerToPeer'],
 'kernel/weavecore/src/virtio_gpu.rs':['VirtioGpuProbe','VIRTIO_GPU_MODERN_DEVICE','bus_master_enabled','initialize_transport','VIRTIO_PCI_CAP_COMMON_CFG'],
 'kernel/weavecore/src/paging.rs':['KERNEL_MMIO_BASE','map_kernel_mmio','PAGE_CACHE_DISABLE'],
 'kernel/weavecore/src/gpu_runtime.rs':['FORGEGRAPHICS_ACCEL_ABI_VERSION','initialize_foundation','initialize_transport','initialize_presentation','initialize_resilience_qualification','present_from_displayd','recover_from_displayd','transport_ready: false','backend=virtio-gpu-modern','[GCOMP]','[RSLN]','[SOAK]','[REBD]'],
 'kernel/weavecore/src/gpu_present.rs':['PRESENT_BUFFER_COUNT','DamageRect','FramePacer','PresentWatchdog'],
 'kernel/weavecore/src/gpu_resilience.rs':['GpuHealthManager','AdapterHealthState','ScanoutTopology','HotplugController','GPU_STALL_RECOVERY_THRESHOLD'],
 'kernel/weavecore/src/native_gpu.rs':['NativeGpuBackend','NativeDriverPhase','NativeIommuReadiness','discover_probe_only','authorize_bus_mastering','HardwareTranslated'],
 'kernel/weavecore/src/translated_dma.rs':['TranslationQualification','IntelVtdHardware','enable_translation','invalidate_iotlb_global','QEMU_EDU_DEVICE'],
 'kernel/weavecore/src/amd_gpu.rs':['AMD_NATIVE_BACKEND_ABI_VERSION','claim_foundation','forgebus::claim_pci_function'],
 'kernel/weavecore/src/native_gpu_binding.rs':['NATIVE_BINDING_ABI_VERSION','GpuMemoryManager','persistent_device_domain'],
 'kernel/weavecore/src/native_gpu_c2.rs':['K14C2_ABI_VERSION','AMD_REQUIRED_FIRMWARE_MASK','AMD_BOOTSTRAP_RING_ENTRIES','actual_gpu_domain_bound'],
 'kernel/weavecore/src/native_gpu_c3.rs':['K14C3_ABI_VERSION','AMD_IP_BRINGUP_ORDER','actual_gpu_domain_bound','command_submission_authorized'],
 'kernel/weavecore/src/native_gpu_c4.rs':['K14C4_ABI_VERSION','RequesterId','requester_domain_planned','persistent_domain_live','bus_master_enabled'],
 'kernel/weavecore/src/native_gpu_c5.rs':['K14C5_ABI_VERSION','AmdViDomainImage','device_table_ready','event_log_ready','persistent_domain_live'],
 'kernel/weavecore/src/native_gpu_c6.rs':['K14C6_ABI_VERSION','AmdViHardwarePlan','hardware_programming_eligible','persistent_domain_live'],
 'kernel/weavecore/src/native_gpu_c7.rs':['K14C7_ABI_VERSION','map_kernel_mmio_readonly','firmware_manifest_ready','gmc_gtt_readiness_planned'],
 'kernel/weavecore/src/native_gpu_c8.rs':['K14C8_ABI_VERSION','VerifiedAsicProfile','safe_read_whitelist_ready','firmware_requirements_resolved'],
}
for rel,tokens in checks.items():
 text=(root/rel).read_text()
 for token in tokens: assert token in text,f'{rel}: {token}'
for app in ['init','logd','console','displayd','archive','trustd','driverd','shell']:
    assert (root/f'userspace/{app}/{app}.S').is_file()
assert (root/'docs/architecture/K12.md').is_file()
assert (root/'docs/architecture/K13.md').is_file()
assert (root/'docs/architecture/K14.md').is_file()
assert (root/'K14_STATUS.md').is_file()
assert (root/'K14A_IMPLEMENTATION.md').is_file()
assert (root/'K14B_IMPLEMENTATION.md').is_file()
assert (root/'K14B_TESTER_GUIDE.md').is_file()
assert (root/'K14C1_IMPLEMENTATION.md').is_file()
assert (root/'K14C1_TESTER_GUIDE.md').is_file()
assert (root/'K14C2_IMPLEMENTATION.md').is_file()
assert (root/'K14C2_TESTER_GUIDE.md').is_file()
assert (root/'K14C3_IMPLEMENTATION.md').is_file()
assert (root/'K14C3_TESTER_GUIDE.md').is_file()
assert (root/'K14C4_IMPLEMENTATION.md').is_file()
assert (root/'K14C4_TESTER_GUIDE.md').is_file()
assert (root/'K14C5_IMPLEMENTATION.md').is_file()
assert (root/'K14C5_TESTER_GUIDE.md').is_file()
assert (root/'K14C6_IMPLEMENTATION.md').is_file()
assert (root/'K14C6_TESTER_GUIDE.md').is_file()
assert (root/'K14C7_IMPLEMENTATION.md').is_file()
assert (root/'K14C7_TESTER_GUIDE.md').is_file()
assert (root/'K14C8_IMPLEMENTATION.md').is_file()
assert (root/'K14C8_TESTER_GUIDE.md').is_file()
assert (root/'K14C9_IMPLEMENTATION.md').is_file()
assert (root/'K14C9_TESTER_GUIDE.md').is_file()
assert (root/'K14C10_IMPLEMENTATION.md').is_file()
assert (root/'K14C10_TESTER_GUIDE.md').is_file()
assert (root/'K14C11_IMPLEMENTATION.md').is_file()
assert (root/'K14C11_TESTER_GUIDE.md').is_file()
assert (root/'K14C12_IMPLEMENTATION.md').is_file()
assert (root/'K14C12_TESTER_GUIDE.md').is_file()
assert (root/'K14C13_IMPLEMENTATION.md').is_file()
assert (root/'K14C13_TESTER_GUIDE.md').is_file()
assert (root/'K14C22_IMPLEMENTATION.md').is_file()
assert (root/'K14C22_TESTER_GUIDE.md').is_file()
assert (root/'K14C23_IMPLEMENTATION.md').is_file()
assert (root/'K14C23_TESTER_GUIDE.md').is_file()
assert (root/'K14C23_SOURCE_STATUS.md').is_file()
assert (root/'K14C24_IMPLEMENTATION.md').is_file()
assert (root/'K14C24_TESTER_GUIDE.md').is_file()
assert (root/'K14C24_SOURCE_STATUS.md').is_file()
assert (root/'K14C25_IMPLEMENTATION.md').is_file()
assert (root/'K14C25_TESTER_GUIDE.md').is_file()
assert (root/'K14C25_SOURCE_STATUS.md').is_file()
assert (root/'K14C25_RUNTIME_QUALIFICATION.md').is_file()
assert (root/'K14C26_IMPLEMENTATION.md').is_file()
assert (root/'K14C26_TESTER_GUIDE.md').is_file()
assert (root/'K14C26_SOURCE_STATUS.md').is_file()
assert (root/'K14_LOCKED_ROADMAP.md').is_file()
assert (root/'K14C27_IMPLEMENTATION.md').is_file()
assert (root/'K14C27_TESTER_GUIDE.md').is_file()
assert (root/'K14C27_SOURCE_STATUS.md').is_file()
assert (root/'K14C28_IMPLEMENTATION.md').is_file()
assert (root/'K14C28_TESTER_GUIDE.md').is_file()
assert (root/'K14C28_SOURCE_STATUS.md').is_file()
assert (root/'K14C29_IMPLEMENTATION.md').is_file()
assert (root/'K14C29_TESTER_GUIDE.md').is_file()
assert (root/'K14C29_SOURCE_STATUS.md').is_file()
assert (root/'K14_TESTER_GUIDE.md').is_file()
assert (root/'K13D_IMPLEMENTATION.md').is_file()
assert (root/'K13D_TESTER_GUIDE.md').is_file()
assert (root/'docs/design/WORKPLACE_SHELL.md').is_file()
assert (root/'docs/design/WORKPLACE_SHELL_REFERENCE.png').stat().st_size > 100_000
assert (root/'docs/design/TITANWEAVE_OS_LOGO.png').stat().st_size > 100_000
PY
if command -v clang >/dev/null 2>&1; then
  t1="$(mktemp --suffix=.o)"; t2="$(mktemp --suffix=.o)"; t3="$(mktemp --suffix=.o)"; trap 'rm -f "$t1" "$t2" "$t3"' EXIT
  clang -c -m64 "$ROOT/kernel/weavecore/src/arch/x86_64/ap_trampoline.S" -o "$t1"
  [[ "$(objdump -h "$t1" | awk '$2==".ap_trampoline"{print $3}')" == "00001000" ]]
  clang -c -m64 "$ROOT/kernel/weavecore/src/arch/x86_64/interrupts.S" -o "$t2"
  clang -c -m64 "$ROOT/kernel/weavecore/src/user_mode.S" -o "$t3"
fi
if command -v clang >/dev/null 2>&1 && command -v ld.lld >/dev/null 2>&1; then
  "$ROOT/tools/build-userspace.sh" >/dev/null
  for app in INIT LOGD CONSOL DISPLAYD ARCHIVE TRUSTD DRIVERD SHELL; do
    elf="$ROOT/build/userspace/$app.ELF"
    readelf -h "$elf" | grep -q 'EXEC (Executable file)'
    readelf -l "$elf" | grep -q 'LOAD'
  done
  python3 "$ROOT/tools/make-fat32.py" --userspace "$ROOT/build/userspace" --output "$ROOT/build/TITANFS.IMG" >/dev/null
  python3 "$ROOT/tools/inspect-fat32.py" "$ROOT/build/TITANFS.IMG" >/dev/null
fi
if command -v cargo >/dev/null 2>&1; then
 cargo metadata --no-deps --format-version 1 --manifest-path "$ROOT/Cargo.toml" >/dev/null
fi
python3 "$ROOT/tools/test-k1-k9-policies.py" >/dev/null
python3 "$ROOT/tools/test-foundation-replacements.py"
python3 "$ROOT/tools/test-k10-policies.py"
python3 "$ROOT/tools/test-k11-policies.py"
python3 "$ROOT/tools/test-k11-prerequisites.py"
python3 "$ROOT/tools/test-k11-runtime-closure.py"
python3 "$ROOT/tools/test-k11-backends.py"
python3 "$ROOT/tools/test-k11-semantic-regressions.py"
python3 "$ROOT/tools/test-k12-graphics.py"
python3 "$ROOT/tools/test-k13-gpu.py"
python3 "$ROOT/tools/test-k13b-transport.py"
python3 "$ROOT/tools/test-k13c-present.py"
python3 "$ROOT/tools/test-k13d-resilience.py"
python3 "$ROOT/tools/test-k14-native-gpu.py"
python3 "$ROOT/tools/test-k14b-translated-dma.py"
python3 "$ROOT/tools/test-k14c1-native-binding.py"
python3 "$ROOT/tools/test-k14c2-native-domain.py"
python3 "$ROOT/tools/test-k14c3-radeon-staging.py"
python3 "$ROOT/tools/test-k14c4-radeon-domain-gate.py"
python3 "$ROOT/tools/test-k14c5-amdvi-page-tables.py"
python3 "$ROOT/tools/test-k14c6-amdvi-live.py"
python3 "$ROOT/tools/test-k14c7-radeon-discovery.py"
python3 "$ROOT/tools/test-k14c8-asic-ip.py"
python3 "$ROOT/tools/test-k14c9-verified-profiles.py"
python3 "$ROOT/tools/test-k14c10-mmio-whitelist.py"
python3 "$ROOT/tools/test-k14c11-reviewed-registers.py"

python3 "$ROOT/tools/test-k14c12-trusted-bases.py"
python3 "$ROOT/tools/test-k14c13-physical-proof.py"
python3 "$ROOT/tools/test-k14c14-write-readiness.py"
python3 "$ROOT/tools/test-k14c15-controlled-write.py"
python3 "$ROOT/tools/test-k14c16-reviewed-mmio-write.py"
python3 "$ROOT/tools/test-k14c17-ip-discovery.py"
python3 "$ROOT/tools/test-k14c18-snapshot-verification.py"
python3 "$ROOT/tools/test-k14c19-physical-snapshot.py"
python3 "$ROOT/tools/test-k14c20-exact-ip-bases.py"
python3 "$ROOT/tools/test-k14c21-reviewed-mmio-rebind.py"
python3 "$ROOT/tools/test-k14c21-elf-build-contract.py"
python3 "$ROOT/tools/test-k14c21-linker-layout.py"
python3 "$ROOT/tools/test-k14c22-reversible-scratch-mutation.py"
python3 "$ROOT/tools/test-k14c23-dual-probe-stability.py"
python3 "$ROOT/tools/test-k14c24-multi-bit-pattern.py"
python3 "$ROOT/tools/test-k14c25-dual-multi-bit-pattern.py"
python3 "$ROOT/tools/test-k14c26-final-mmio-allowlist.py"
python3 "$ROOT/tools/test-k14c27-radeon-driver-core.py"
python3 "$ROOT/tools/test-k14c28-memory-firmware-recovery.py"
python3 "$ROOT/tools/test-k14c29-rings-queues-fences-dma.py"
echo "Titanweave K1-K14.C29 integrated source validation passed; K14.C29 runtime qualification pending."
