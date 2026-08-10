#![no_main]
#![no_std]

mod sha256;
mod framebuffer;
mod graphics_abi;
mod forgegraphics;
mod forgeaudio;
mod forgeaudio_dma;
mod gpu_topology;
mod gpu_memory;
mod gpu_queue;
mod gpu_fence;
mod gpu_modeset;
mod gpu_multigpu;
mod virtio_gpu;
mod gpu_runtime;
mod gpu_present;
mod gpu_resilience;
mod native_gpu;
mod amd_gpu;
mod native_gpu_binding;
mod native_gpu_c2;
mod native_gpu_c3;
mod native_gpu_c4;
mod native_gpu_c5;
mod native_gpu_c6;
mod native_gpu_c7;
mod native_gpu_c8;
mod native_gpu_c9;
mod native_gpu_c10;
mod native_gpu_c11;
mod native_gpu_c12;
mod native_gpu_c13;
mod native_gpu_c14;
mod native_gpu_c15;
mod native_gpu_c16;
mod native_gpu_c17;
mod native_gpu_c18;
mod native_gpu_c19;
mod native_gpu_c20;
mod native_gpu_c21;
mod native_gpu_c22;
mod native_gpu_c23;
mod native_gpu_c24;
mod native_gpu_c25;
mod native_gpu_c26;
mod radeon_mmio;
mod radeon_resources;
mod radeon_driver;
mod native_gpu_c27;
mod radeon_memory;
mod radeon_firmware;
mod radeon_recovery;
mod native_gpu_c28;
mod native_gpu_c29;
mod radeon_edid;
mod radeon_dcn401;
mod radeon_display;
mod native_gpu_c30;
mod radeon_shader;
mod radeon_shader_cache;
mod radeon_command;
mod radeon_pipeline;
mod radeon_compute;
mod radeon_compute_caps;
mod radeon_graphics;
mod native_gpu_c31;
mod radeon_telemetry;
mod radeon_power;
mod radeon_multigpu;
mod radeon_gpu_abi;
mod radeon_stability;
mod native_gpu_c32;
mod radeon_sdma_packets;
mod radeon_ring;
mod radeon_queue;
mod radeon_fence;
mod radeon_dma;
mod radeon_sdma;
mod translated_dma;
mod compositor;
mod input_router;
mod display;
mod workplace_shell;
mod device;
mod driver;
mod dma;
mod iommu;
mod amd_vi;
mod intel_vtd;
mod iommu_core;
mod iova;
mod pci_address;
mod pci_ecam;
mod msi;
mod xhci;
mod usb_hid_full;
mod nvme_full;
mod pcie_hotplug;
mod k11_stress;
mod k11_backends;
mod interrupt_router;
mod hotplug;
mod nvme;
mod usb_hid;
mod forgebus;
mod capability;
mod trust;
mod update;
mod trust_service;
mod abi;
mod archive;
mod archive_service;
mod block;
mod driver_watchdog;
mod kernel_runtime;
mod object_lifecycle;
mod block_queue;
mod acpi;
mod automount;
mod arch;
mod elf;
mod fat32;
mod gpt;
mod ntfs;
mod recovery;
mod storage;
mod titan_cache;
mod volume;
mod volume_events;
mod mount_namespace;
mod handles;
mod ipc;
mod memory;
mod namespace;
mod objects;
mod paging;
mod pci;
mod process;
mod package;
mod percpu;
mod scheduler;
mod rt_mutex;
mod service;
mod serial;
mod shared_memory;
mod sync;
mod syscalls;
mod user;
mod vfs;
mod virtio_blk;

use crate::arch::x86_64;
use crate::arch::x86_64::{apic, gdt, halt_forever, idt, pit, smp};
use crate::memory::{bytes_to_mib, BumpHeap, FrameAllocator, FRAME_SIZE};
use core::arch::asm;
use core::panic::PanicInfo;
use titanweave_boot_protocol::{BootInfo, BOOT_INFO_MAGIC, BOOT_PROTOCOL_VERSION};

const EARLY_HEAP_PAGES: u64 = 256;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn weavecore_entry(boot_info_address: u64) -> ! {
    serial::initialize();
    serial::println(format_args!(""));
    serial::println(format_args!("TITAN//WEAVE"));
    serial::println(format_args!(
        "[BOOT] WeaveCore K14 entered from WEAVECORE.ELF"
    ));

    let boot_info = unsafe { &*(boot_info_address as *const BootInfo) };
    if boot_info.magic != BOOT_INFO_MAGIC
        || boot_info.protocol_version != BOOT_PROTOCOL_VERSION
        || !boot_info.is_structurally_valid()
    {
        serial::println(format_args!("[FAIL] Invalid BootInfo v14 handoff"));
        halt_forever();
    }

    serial::println(format_args!(
        "[BOOT] Protocol v{}; BootInfo {} bytes; RSDP={:#x}",
        boot_info.protocol_version,
        boot_info.structure_size,
        boot_info.acpi.rsdp_address
    ));
    serial::println(format_args!(
        "[MMU ] CR3={:#018x}; identity map through {} GiB",
        boot_info.bootstrap.page_table_root,
        boot_info.bootstrap.identity_map_limit / (1024 * 1024 * 1024)
    ));

    let execution_state = x86_64::initialize_execution_state();
    serial::println(format_args!(
        "[CPU ] execution state: CR0 {:#018x}->{:#018x}; CR4 {:#018x}->{:#018x}; CET={}",
        execution_state.cr0_before,
        execution_state.cr0_after,
        execution_state.cr4_before,
        execution_state.cr4_after,
        if execution_state.cr4_before & (1 << 23) != 0 { "disabled" } else { "off" }
    ));

    // CPU 0 is a safe bootstrap slot until ACPI tells us the BSP's logical
    // topology index. The correct per-CPU table is reloaded after MADT parsing.
    gdt::load_for_cpu(0);
    serial::println(format_args!(
        "[CPU ] Bootstrap GDT/TSS and emergency IST stacks installed"
    ));
    idt::initialize();
    serial::println(format_args!("[CPU ] Kernel IDT installed"));

    unsafe { asm!("int3", options(nomem, nostack)) };
    serial::println(format_args!(
        "[TEST] Breakpoint exception returned successfully"
    ));

    let summary = memory::summarize(boot_info);
    serial::println(format_args!(
        "[MEM ] {} descriptors; {} MiB described; {} MiB conventional",
        boot_info.memory_map.descriptor_count,
        bytes_to_mib(summary.total_pages.saturating_mul(FRAME_SIZE)),
        bytes_to_mib(summary.conventional_pages.saturating_mul(FRAME_SIZE))
    ));

    let mut allocator = FrameAllocator::new(boot_info);
    let Some(first_frame) = allocator.allocate_frame() else {
        serial::println(format_args!("[FAIL] No conventional memory frame available"));
        halt_forever();
    };
    serial::println(format_args!(
        "[MEM ] First free 4 KiB frame: {first_frame:#018x}"
    ));

    let Some(heap_base) = allocator.allocate_contiguous(EARLY_HEAP_PAGES) else {
        serial::println(format_args!("[FAIL] Could not reserve K13 early heap"));
        halt_forever();
    };
    let mut heap =
        BumpHeap::new(heap_base, EARLY_HEAP_PAGES * FRAME_SIZE).expect("early heap overflow");
    let small = heap
        .allocate(96, 16)
        .expect("early heap small allocation failed");
    let page = heap
        .allocate(4096, 4096)
        .expect("early heap page allocation failed");
    serial::println(format_args!(
        "[HEAP] {} MiB bump heap online at {:#x}; test blocks={:#x},{:#x}; used={} bytes",
        bytes_to_mib(heap.capacity()),
        heap_base,
        small,
        page,
        heap.used()
    ));

    let platform = match acpi::discover(
        boot_info.acpi.rsdp_address,
        boot_info.bootstrap.identity_map_limit,
    ) {
        Ok(platform) => platform,
        Err(error) => {
            serial::println(format_args!("[FAIL] ACPI discovery failed: {error}"));
            halt_forever();
        }
    };
    serial::println(format_args!(
        "[ACPI] MADT found; local APIC={:#x}; logical CPUs={}",
        platform.local_apic_address, platform.cpu_count
    ));
    for index in 0..platform.cpu_count {
        serial::println(format_args!(
            "[ACPI] CPU {} uses APIC ID {}",
            index, platform.apic_ids[index]
        ));
    }

    let bsp_apic_id = apic::initialize(platform.local_apic_address);
    let bsp_index = platform
        .logical_index_for_apic(bsp_apic_id)
        .unwrap_or(0);
    percpu::initialize(bsp_index, bsp_apic_id, true);
    gdt::load_for_cpu(bsp_index);
    idt::load();
    serial::println(format_args!(
        "[APIC] BSP local APIC online: logical={} APIC={}",
        bsp_index, bsp_apic_id
    ));

    let smp_report = smp::start_application_processors(
        boot_info,
        &platform,
        bsp_apic_id,
        &mut allocator,
    );
    serial::println(format_args!(
        "[SMP ] discovered={} online={} failed={} per_cpu_online={}",
        smp_report.discovered,
        smp_report.online,
        smp_report.failed,
        percpu::online_count()
    ));

    let timer_initial_count = match apic::calibrate_timer_100hz(idt::TIMER_VECTOR) {
        Ok(count) => count,
        Err(error) => {
            serial::println(format_args!("[FAIL] APIC timer calibration failed: {error}"));
            halt_forever();
        }
    };
    serial::println(format_args!(
        "[TIME] Local APIC timer calibrated: {} counts per {} ms / 100 Hz tick",
        timer_initial_count,
        pit::calibration_milliseconds()
    ));

    let scheduler_report = match scheduler::run_scheduler_self_test(
        &mut allocator,
        bsp_index,
        timer_initial_count,
    ) {
        Ok(report) => report,
        Err(error) => {
            serial::println(format_args!("[FAIL] K13 scheduler foundation failed: {error}"));
            halt_forever();
        }
    };

    serial::println(format_args!(
        "[SCHED] K3 foundation retained: ticks={} preemptions={} completed_tasks={}",
        scheduler_report.ticks,
        scheduler_report.preemptions,
        scheduler_report.tasks_completed
    ));

    let forgeaudio_rt_report = match scheduler::run_forgeaudio_rt_self_test(
        &mut allocator,
        bsp_index,
        timer_initial_count,
    ) {
        Ok(report) => report,
        Err(error) => {
            serial::println(format_args!("[FAIL] K15.1 ForgeAudio RT execution foundation failed: {error}"));
            halt_forever();
        }
    };
    serial::println(format_args!(
        "[K15RD] ForgeAudio RT ready: tick_hz={} cpu={} jobs={} misses={} budget_exhaustions={} PI={} guard_deferrals={} reserved={}",
        forgeaudio_rt_report.tick_hz,
        scheduler::audio_reserved_cpu().unwrap_or(bsp_index),
        forgeaudio_rt_report.audio_jobs_completed,
        forgeaudio_rt_report.deadline_misses,
        forgeaudio_rt_report.budget_exhaustions,
        forgeaudio_rt_report.priority_inheritance_events,
        forgeaudio_rt_report.preemption_deferrals,
        forgeaudio_rt_report.audio_cpu_reserved,
    ));

    let forgeaudio_abi = match forgeaudio::initialize() {
        Ok(info) => info,
        Err(error) => {
            serial::println(format_args!("[FAIL] K15.2 ForgeAudio kernel ABI initialization failed: {error}"));
            halt_forever();
        }
    };
    if let Err(error) = forgeaudio::run_abi_self_test() {
        serial::println(format_args!("[FAIL] K15.2 ForgeAudio kernel ABI qualification failed: {error}"));
        halt_forever();
    }
    serial::println(format_args!(
        "[K15ARD] ForgeAudio ABI ready: version={} features={:#x} real_devices={} fake_devices=false",
        forgeaudio_abi.current_version,
        forgeaudio_abi.features,
        forgeaudio::device_count(),
    ));

    match display::initialize(boot_info) {
        Ok(state) => serial::println(format_args!(
            "[DISP] K14 display foundation ready: {}x{} firmware_fallback={}",
            state.width, state.height, state.firmware_fallback
        )),
        Err(error) => {
            display::initialize_headless();
            serial::println(format_args!("[DISP] K14 display fallback unavailable: {error}"));
        }
    }
    serial::println(format_args!(
        "[MOD ] Boot modules available: {}",
        boot_info.modules.count
    ));
    for module in boot_info.modules.iter() {
        serial::println(format_args!(
            "[MOD ] {} kind={} address={:#x} bytes={}",
            core::str::from_utf8(module.name_bytes()).unwrap_or("module"),
            module.kind,
            module.physical_address,
            module.byte_size
        ));
    }

    let recovery_health = recovery::initialize();
    serial::println(format_args!(
        "[RECV] boot health: failed_boots={} recovery_required={} cache_reset_required={}",
        recovery_health.failed_boots,
        recovery_health.recovery_required,
        recovery_health.cache_reset_required
    ));
    let cache_policy = titan_cache::initialize(summary.conventional_pages.saturating_mul(FRAME_SIZE));
    serial::println(format_args!(
        "[CACHE] TitanCache read cache={} MiB write_back={} preload={}",
        bytes_to_mib(cache_policy.maximum_bytes),
        cache_policy.write_back_enabled,
        cache_policy.preload_enabled
    ));
    match storage::initialize(boot_info) {
        Ok(report) => serial::println(format_args!(
            "[STOR] K11 auto-mount retained: sectors={} discovered={} mounted={} read_only={} hidden={} quarantined={}",
            report.sectors, report.discovered, report.mounted, report.read_only, report.hidden, report.quarantined
        )),
        Err(error) => serial::println(format_args!("[STOR] K10 auto-mount failed: {error}")),
    }

    archive_service::initialize();
    trust_service::initialize();

    if let Err(error) = forgebus::initialize() {
        serial::println(format_args!("[BUS ] ForgeBus initialization failed: {error}"));
    }
    match acpi::AcpiCatalog::build(boot_info.acpi.rsdp_address, boot_info.bootstrap.identity_map_limit) {
        Ok(catalog) => {
            if let Err(error) = k11_backends::initialize(&catalog) { serial::println(format_args!("[FAIL] K11 backend initialization failed: {error}")); halt_forever(); }
        }
        Err(error) => { serial::println(format_args!("[FAIL] ACPI catalog failed: {error}")); halt_forever(); }
    }

    match gpu_runtime::initialize_foundation() {
        Ok(state) => serial::println(format_args!(
            "[GPUF] K13 acceleration foundation ready: adapters={} native={} virtio={} transport_ready={}",
            state.adapters, state.native_adapters, state.virtio_candidates, state.transport_ready
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K13 GPU acceleration foundation failed: {error}"));
            halt_forever();
        }
    }

    match gpu_runtime::initialize_transport(
        &mut allocator,
        boot_info.bootstrap.page_table_root,
        boot_info.bootstrap.identity_map_limit,
    ) {
        Ok(state) if state.transport_ready => serial::println(format_args!(
            "[GPUT] K13.B VirtIO-GPU transport ready: {}x{} controlq={} cursorq={}",
            state.scanout_width, state.scanout_height, state.control_queue_size, state.cursor_queue_size
        )),
        Ok(_) => serial::println(format_args!(
            "[GPUT] K13.B transport not activated; firmware/native fallback retained"
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K13.B VirtIO-GPU transport failed: {error}"));
            halt_forever();
        }
    }

    match gpu_runtime::initialize_presentation(&mut allocator) {
        Ok(state) if state.presentation_ready => serial::println(format_args!(
            "[GPRE] K13.C buffered presentation ready: buffers={} frames={} fence={}",
            state.present_buffers, state.presented_frames, state.last_present_fence
        )),
        Ok(_) => {
            serial::println(format_args!("[FAIL] K13.C presentation did not become ready"));
            halt_forever();
        }
        Err(error) => {
            serial::println(format_args!("[FAIL] K13.C compositor presentation failed: {error}"));
            halt_forever();
        }
    }

    match gpu_runtime::initialize_resilience_qualification() {
        Ok(state) if state.resilience_ready => serial::println(format_args!(
            "[GRDY] K13.D resilience/multi-GPU ready: secondary={} soak_frames={} recoveries={}",
            state.secondary_candidates, state.soak_frames, state.recovery_cycles
        )),
        Ok(_) => {
            serial::println(format_args!("[FAIL] K13.D resilience path did not become ready"));
            halt_forever();
        }
        Err(error) => {
            serial::println(format_args!("[FAIL] K13.D resilience/multi-GPU qualification failed: {error}"));
            halt_forever();
        }
    }

    match translated_dma::initialize_qualification(
        &mut allocator,
        boot_info.bootstrap.page_table_root,
    ) {
        Ok(state) => serial::println(format_args!(
            "[IOMF] K14.B translated DMA qualification ready: backend={:?} translated={} mappings={} blocked_faults={} invalidations={}",
            state.backend,
            state.hardware_translated,
            state.mappings_verified,
            state.blocked_faults,
            state.invalidations
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.B hardware translation qualification failed: {error}"));
            halt_forever();
        }
    }

    let forgeaudio_dma_report = match forgeaudio_dma::run_self_test(
        &mut allocator,
        boot_info.bootstrap.page_table_root,
    ) {
        Ok(report) => report,
        Err(error) => {
            serial::println(format_args!("[FAIL] K15.3 ForgeAudio audio DMA transport qualification failed: {error}"));
            halt_forever();
        }
    };
    serial::println(format_args!(
        "[K15DR] ForgeAudio DMA ready: version={} real_memory={} periods={} wraps={} underruns={} overruns={} translated_platform={} qemu_hda_deferred={}",
        forgeaudio_dma_report.version,
        forgeaudio_dma_report.real_dma_memory,
        forgeaudio_dma_report.completed_playback_periods,
        forgeaudio_dma_report.playback_wraps,
        forgeaudio_dma_report.playback_underruns,
        forgeaudio_dma_report.capture_overruns,
        forgeaudio_dma_report.translated_platform_qualified,
        forgeaudio_dma_report.hardware_audio_deferred,
    ));

    match native_gpu::initialize_foundation() {
        Ok(state) => serial::println(format_args!(
            "[NATF] K14.A native GPU prerequisite foundation ready: adapters={} activation_ready={} iommu={:?}",
            state.adapters, state.activation_ready, state.iommu_readiness
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.A native GPU prerequisite foundation failed: {error}"));
            halt_forever();
        }
    }

    match native_gpu_binding::initialize_binding_foundation() {
        Ok(state) => serial::println(format_args!(
            "[NCF ] K14.C1 native binding foundation ready: candidates={} selected_vendor={} claimed={} persistent_domain={} bus_master={} fallback={}",
            state.candidates, state.selected_vendor, state.forge_claimed,
            state.persistent_device_domain, state.bus_master_enabled, state.fallback_armed
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.C1 native GPU binding foundation failed: {error}"));
            halt_forever();
        }
    }

    match native_gpu_c2::initialize(&mut allocator, boot_info.bootstrap.page_table_root) {
        Ok(state) => serial::println(format_args!(
            "[C2NF] K14.C2 native persistent-domain/AMD bring-up ready: surrogate_domain={} epochs={} amd_candidate={} actual_gpu_domain={} bus_master={} fallback={}",
            state.surrogate_domain_qualified, state.persistent_epochs, state.amd_candidate,
            state.actual_gpu_domain_bound, state.bus_master_enabled, state.fallback_armed
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.C2 native bring-up contract failed: {error}"));
            halt_forever();
        }
    }

    match native_gpu_c3::initialize() {
        Ok(state) => serial::println(format_args!(
            "[C3NF] K14.C3 Radeon bare-metal staging ready: amd_present={} actual_domain={} mmio={} firmware={} submit={} bus_master={} fallback={}",
            state.amd_present, state.actual_gpu_domain_bound, state.mmio_mapping_authorized,
            state.firmware_upload_authorized, state.command_submission_authorized,
            state.bus_master_enabled, state.fallback_armed
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.C3 Radeon bare-metal staging failed: {error}"));
            halt_forever();
        }
    }

    match native_gpu_c4::initialize() {
        Ok(state) => serial::println(format_args!(
            "[C4NF] K14.C4 Radeon exact-domain qualification ready: amd_present={} amd_vi={} domain_planned={} domain_live={} bus_master={} fallback={}",
            state.amd_present, state.amd_vi_active, state.requester_domain_planned,
            state.persistent_domain_live, state.bus_master_enabled, state.fallback_armed
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.C4 Radeon exact-domain qualification failed: {error}"));
            halt_forever();
        }
    }

    match native_gpu_c5::initialize(&mut allocator) {
        Ok(state) => serial::println(format_args!(
            "[C5NF] K14.C5 AMD-Vi page-table engine ready: amd_present={} tables={} dte={} cmd={} event={} fault={} exact_bound={} domain_live={} read_mmio={} bus_master={} fallback={}",
            state.amd_present, state.page_tables_ready, state.device_table_ready,
            state.command_buffer_ready, state.event_log_ready, state.fault_path_ready,
            state.exact_requester_bound, state.persistent_domain_live,
            state.mmio_read_mapping_allowed, state.bus_master_enabled, state.fallback_armed
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.C5 AMD-Vi page-table engine failed: {error}"));
            halt_forever();
        }
    }

    match native_gpu_c6::initialize(&mut allocator, boot_info.bootstrap.page_table_root) {
        Ok(state) => serial::println(format_args!(
            "[C6NF] K14.C6 live AMD-Vi engine ready: amd_present={} eligible={} programmed={} translation={} domain_live={} read_mmio={} bus_master={} fallback={}",
            state.amd_present, state.hardware_programming_eligible, state.hardware_programmed,
            state.translation_enabled, state.persistent_domain_live, state.read_only_radeon_mmio_promoted,
            state.bus_master_enabled, state.fallback_armed
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.C6 live AMD-Vi engine failed: {error}"));
            halt_forever();
        }
    }

    match native_gpu_c7::initialize(&mut allocator, boot_info.bootstrap.page_table_root) {
        Ok(state) => serial::println(format_args!(
            "[C7NF] K14.C7 Radeon discovery ready: amd_present={} domain_live={} pci_identity={} ro_mmio={} firmware_manifest={} gmc_gtt_plan={} bus_master={} fallback={}",
            state.amd_present, state.exact_domain_live, state.pci_identity_ready,
            state.read_only_mmio_mapped, state.firmware_manifest_ready, state.gmc_gtt_readiness_planned,
            state.bus_master_enabled, state.fallback_armed
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.C7 Radeon discovery failed: {error}"));
            halt_forever();
        }
    }

    match native_gpu_c8::initialize() {
        Ok(state) => serial::println(format_args!(
            "[C8NF] K14.C8 Radeon ASIC/IP identification ready: amd_present={} ro_mmio={} profile_verified={} ip_manifest={} whitelist={} safe_reads={} firmware_resolved={} gmc_gtt={} bus_master={} fallback={}",
            state.amd_present, state.c7_ro_mmio_ready, state.asic_profile_verified, state.ip_manifest_ready,
            state.safe_read_whitelist_ready, state.safe_register_reads_enabled, state.firmware_requirements_resolved,
            state.gmc_gtt_init_ready, state.bus_master_enabled, state.fallback_armed
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.C8 Radeon ASIC/IP identification failed: {error}"));
            halt_forever();
        }
    }

    match native_gpu_c9::initialize() {
        Ok(state) => serial::println(format_args!(
            "[C9NF] K14.C9 verified Radeon profiles ready: amd_present={} profile_verified={} pci_reads={} identity_consistent={} mmio_whitelist={} mmio_reads={} firmware_resolved={} gmc_gtt_profile={} bus_master={} fallback={}",
            state.amd_present, state.profile_verified, state.safe_pci_reads_performed, state.pci_identity_consistent,
            state.mmio_whitelist_ready, state.mmio_register_reads_enabled, state.firmware_requirements_resolved,
            state.gmc_gtt_profile_ready, state.bus_master_enabled, state.fallback_armed
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.C9 verified Radeon profile/live-read gate failed: {error}"));
            halt_forever();
        }
    }

    match native_gpu_c10::initialize() {
        Ok(state) => serial::println(format_args!(
            "[C10OK] K14.C10 guarded MMIO-read engine: amd_present={} profile_verified={} ro_mmio={} whitelist_profile={} whitelist_reviewed={} entries={} live_reads={} performed={} bus_master={} fallback={}",
            state.amd_present, state.profile_verified, state.read_only_aperture_ready, state.whitelist_profile_found,
            state.whitelist_reviewed, state.whitelist_entries, state.live_mmio_reads_enabled,
            state.live_mmio_reads_performed, state.bus_master_enabled, state.fallback_armed
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.C10 per-IP MMIO whitelist/live-read gate failed: {error}"));
            halt_forever();
        }
    }

    match native_gpu_c11::initialize() {
        Ok(state) => serial::println(format_args!(
            "[C11OK] K14.C11 reviewed register/IP-base gate: amd_present={} profile_verified={} reviewed_profile={} entries={} definitions_reviewed={} ip_bases={} address_translation={} live_reads={} performed={} bus_master={} fallback={}",
            state.amd_present, state.profile_verified, state.reviewed_profile_found, state.reviewed_entries,
            state.register_definitions_reviewed, state.ip_base_map_ready, state.address_translation_verified,
            state.live_mmio_reads_enabled, state.live_mmio_reads_performed, state.bus_master_enabled, state.fallback_armed
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.C11 reviewed register/IP-base gate failed: {error}"));
            halt_forever();
        }
    }

    match native_gpu_c12::initialize(&mut allocator, boot_info.bootstrap.page_table_root) {
        Ok(state) => serial::println(format_args!(
            "[C12OK] K14.C12 trusted IP-base/live-read engine: amd_present={} profile_verified={} domain_live={} base_source={:?} gc_base={} sdma_base={} bar5={} live_gate={} reads={} bus_master={} fallback={}",
            state.amd_present, state.profile_verified, state.exact_domain_live, state.base_source,
            state.gc_base_ready, state.sdma_base_ready, state.register_mmio_bar_ready, state.live_read_gate_ready,
            state.live_reads_performed, state.bus_master_enabled, state.fallback_armed
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.C12 trusted IP-base/live-read gate failed: {error}"));
            halt_forever();
        }
    }

    match native_gpu_c13::initialize() {
        Ok(state) => serial::println(format_args!(
            "[C13OK] K14.C13 physical Radeon read-proof engine: amd_present={} profile_verified={} c12_proof={} sane={} bus_master_off={} physical_proof={} navi48_discovery_pending={} reads={} fingerprint={:#018x} fallback={}",
            state.amd_present, state.profile_verified, state.c12_live_read_proof, state.read_values_sane,
            state.bus_master_rechecked_off, state.physical_proof_complete, state.navi48_discovery_pending,
            state.reads_inherited, state.proof_fingerprint, state.fallback_armed
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.C13 physical Radeon read-proof gate failed: {error}"));
            halt_forever();
        }
    }

    match native_gpu_c14::initialize() {
        Ok(state) => serial::println(format_args!(
            "[C14OK] K14.C14 controlled write-promotion readiness gate: amd_present={} profile={} domain={} trusted_bases={} bar5={} physical_proof={} fingerprint={} bus_master_off={} prerequisites={} promotion={} navi48_pending={} fallback={}",
            state.amd_present, state.profile_verified, state.exact_domain_live, state.trusted_base_source,
            state.register_bar_ready, state.physical_read_proof, state.proof_fingerprint_present,
            state.bus_master_rechecked_off, state.write_prerequisites_complete, state.write_promotion_enabled,
            state.navi48_discovery_pending, state.fallback_armed
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.C14 write-promotion readiness gate failed: {error}"));
            halt_forever();
        }
    }

    match native_gpu_c15::initialize() {
        Ok(state) => serial::println(format_args!(
            "[C15OK] K14.C15 controlled write transaction: amd_present={} C14_prerequisites={} eligible={} attempted={} verified={} rollback_attempted={} rollback_verified={} bus_master_before_off={} bus_master_after_off={} attempts={} fingerprint={:#018x} MMIO_writes=false fallback={}",
            state.amd_present, state.c14_prerequisites_complete, state.transaction_eligible,
            state.identity_write_attempted, state.identity_write_verified, state.rollback_attempted,
            state.rollback_verified, state.bus_master_before_off, state.bus_master_after_off,
            state.write_attempts, state.transaction_fingerprint, state.fallback_armed
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14.C15 controlled write transaction failed: {error}"));
            halt_forever();
        }
    }

    match native_gpu_c16::initialize(&mut allocator, boot_info.bootstrap.page_table_root) {
        Ok(state) => serial::println(format_args!(
            "[C16OK] K14.C16 reviewed Radeon MMIO-write gate: amd_present={} target={:?} reviewed={} resolved={} trusted_base={} bar5={} attempted={} verified={} writes={} fallback={}",
            state.amd_present,state.target,state.target_reviewed,state.target_resolved,state.trusted_base_ready,state.bar5_ready,state.transaction_attempted,state.transaction_verified,state.writes_performed,state.fallback_armed
        )),
        Err(error) => { serial::println(format_args!("[FAIL] K14.C16 reviewed Radeon MMIO-write gate failed: {error}")); halt_forever(); }
    }

    match native_gpu_c17::initialize() {
        Ok(state) => serial::println(format_args!(
            "[C17OK] K14.C17 AMD IP-discovery/Navi48 base-resolution gate: amd_present={} navi48={} parser={} snapshot={} verified={} gc_base={} promotable={} fallback={}",
            state.amd_present,state.navi48,state.parser_ready,state.live_snapshot_available,state.live_snapshot_verified,state.exact_gc_base_resolved,state.c16_target_promotable,state.fallback_armed
        )),
        Err(error) => { serial::println(format_args!("[FAIL] K14.C17 AMD IP-discovery gate failed: {error}")); halt_forever(); }
    }

    match native_gpu_c18::initialize() {
        Ok(state) => serial::println(format_args!(
            "[C18OK] K14.C18 AMD discovery snapshot-verification gate: amd_present={} navi48={} checksum_engine={} TMR_contract={} acquisition={} snapshot={} binary_ck={} ip_ck={} verified={} fallback={}",
            state.amd_present,state.navi48,state.checksum_engine_ready,state.tmr_contract_imported,state.acquisition_promoted,state.live_snapshot_acquired,state.live_binary_checksum_verified,state.live_ip_checksum_verified,state.live_snapshot_verified,state.fallback_armed
        )),
        Err(error) => { serial::println(format_args!("[FAIL] K14.C18 AMD discovery snapshot-verification gate failed: {error}")); halt_forever(); }
    }

    match native_gpu_c19::initialize(
        &mut allocator,
        boot_info.bootstrap.page_table_root,
        boot_info.acpi.rsdp_address,
        boot_info.bootstrap.identity_map_limit,
    ) {
        Ok(state) => serial::println(format_args!(
            "[C19OK] K14.C19 physical AMD discovery snapshot gate: amd_present={} navi48={} C18_ready={} domain={} profile={} bar5={} scratch_reads={} ECAM={} rebar={} aperture={} covers={} acquired={} verified={} bytes={} fingerprint={:#018x} fallback={}",
            state.amd_present,state.navi48,state.c18_ready,state.exact_domain_live,state.profile_verified,state.bar5_ready,state.scratch_reads_performed,state.ecam_ready,state.rebar_found,state.bar0_aperture_bytes,state.aperture_covers_tmr,state.live_snapshot_acquired,state.live_snapshot_verified,state.snapshot_bytes,state.snapshot_fingerprint,state.fallback_armed
        )),
        Err(error) => { serial::println(format_args!("[FAIL] K14.C19 physical AMD discovery snapshot gate failed: {error}")); halt_forever(); }
    }

    match native_gpu_c20::initialize() {
        Ok(state) => serial::println(format_args!(
            "[C20OK] K14.C20 AMD exact live IP-base gate: amd_present={} navi48={} C19_verified={} parser={} records={} ip_v={} base64={} GC={} GC_base={:#x} SDMA0={} SDMA0_base={:#x} exact_base_set={} c16_input={} fingerprint={:#018x} fallback={}",
            state.amd_present,state.navi48,state.c19_snapshot_verified,state.parser_ready,state.records_scanned,state.ip_version,state.base_addr_64_bit,state.gc_resolved,state.gc_base_dwords,state.sdma0_resolved,state.sdma0_base_dwords,state.exact_base_set_ready,state.c16_promotion_input_ready,state.snapshot_fingerprint,state.fallback_armed
        )),
        Err(error) => { serial::println(format_args!("[FAIL] K14.C20 AMD exact live IP-base gate failed: {error}")); halt_forever(); }
    }

    match native_gpu_c21::initialize(&mut allocator, boot_info.bootstrap.page_table_root) {
        Ok(state) => serial::println(format_args!(
            "[C21OK] K14.C21 reviewed GFX12 target rebind/identity-write gate: amd_present={} navi48={} profile={} domain={} C16_reviewed={} C19_verified={} C20_ready={} GC_base1={} crosscheck={} target={:#x} BAR5={} memdecode={} eligible={} attempted={} verified={} writes={} polls={} fingerprint={:#018x} fallback={}",
            state.amd_present,state.navi48,state.profile_verified,state.exact_domain_live,state.c16_target_reviewed,state.c19_snapshot_verified,state.c20_exact_bases_ready,state.gc_segment1_resolved,state.gc_segment0_crosschecked,state.target_dword_offset,state.bar5_ready,state.memory_decode_before_on,state.transaction_eligible,state.transaction_attempted,state.transaction_verified,state.writes_performed,state.readback_polls,state.transaction_fingerprint,state.fallback_armed
        )),
        Err(error) => { serial::println(format_args!("[FAIL] K14.C21 reviewed GFX12 target rebind/identity-write gate failed: {error}")); halt_forever(); }
    }

    match native_gpu_c22::initialize(&mut allocator, boot_info.bootstrap.page_table_root) {
        Ok(state) => serial::println(format_args!(
            "[C22OK] K14.C22 reversible GFX12 SCRATCH_REG0 mutation gate: amd_present={} navi48={} profile={} domain={} C21_identity={} C21_target={} revalidated={} target={:#x} BAR5={} memdecode={} eligible={} attempted={} mutation={} restored={} retry={} writes={} mutation_polls={} restore_polls={} fingerprint={:#018x} fallback={}",
            state.amd_present,state.navi48,state.profile_verified,state.exact_domain_live,state.c21_identity_verified,state.c21_target_reused,state.target_revalidated,state.target_dword_offset,state.bar5_ready,state.memory_decode_before_on,state.transaction_eligible,state.mutation_attempted,state.mutation_verified,state.restore_verified,state.restore_retry_used,state.writes_performed,state.mutation_polls,state.restore_polls,state.transaction_fingerprint,state.fallback_armed
        )),
        Err(error) => { serial::println(format_args!("[FAIL] K14.C22 reversible GFX12 scratch mutation gate failed: {error}")); halt_forever(); }
    }

    match native_gpu_c23::initialize(&mut allocator, boot_info.bootstrap.page_table_root) {
        Ok(state) => serial::println(format_args!(
            "[C23OK] K14.C23 GFX12 SCRATCH_REG0 persistence/dual-probe stability gate: amd_present={} navi48={} profile={} domain={} C22_mutation={} C22_restore={} target={} C22_persisted={} intercycle={} eligible={} cycleA={} restoreA={} cycleB={} restoreB={} dual={} writes={} fingerprint={:#018x} fallback={}",
            state.amd_present,state.navi48,state.profile_verified,state.exact_domain_live,state.c22_mutation_verified,state.c22_restore_verified,state.target_revalidated,state.c22_restore_persisted,state.intercycle_restore_persisted,state.transaction_eligible,state.cycle_a_mutation_verified,state.cycle_a_restore_verified,state.cycle_b_mutation_verified,state.cycle_b_restore_verified,state.dual_cycle_verified,state.writes_performed,state.transaction_fingerprint,state.fallback_armed
        )),
        Err(error) => { serial::println(format_args!("[FAIL] K14.C23 GFX12 scratch persistence/dual-probe stability gate failed: {error}")); halt_forever(); }
    }

    match native_gpu_c24::initialize(&mut allocator, boot_info.bootstrap.page_table_root) {
        Ok(state) => serial::println(format_args!(
            "[C24OK] K14.C24 reversible GFX12 SCRATCH_REG0 multi-bit pattern gate: amd_present={} navi48={} profile={} domain={} C23_dual={} C23_target={} revalidated={} C23_persisted={} target={:#x} BAR5={} eligible={} attempted={} pattern={} restored={} retry={} writes={} pattern_polls={} restore_polls={} fingerprint={:#018x} fallback={}",
            state.amd_present,state.navi48,state.profile_verified,state.exact_domain_live,state.c23_dual_cycle_verified,state.c23_target_revalidated,state.target_revalidated,state.c23_restore_persisted,state.target_dword_offset,state.bar5_ready,state.transaction_eligible,state.pattern_attempted,state.pattern_verified,state.restore_verified,state.restore_retry_used,state.writes_performed,state.pattern_polls,state.restore_polls,state.transaction_fingerprint,state.fallback_armed
        )),
        Err(error) => { serial::println(format_args!("[FAIL] K14.C24 reversible GFX12 multi-bit pattern gate failed: {error}")); halt_forever(); }
    }

    match native_gpu_c25::initialize(&mut allocator, boot_info.bootstrap.page_table_root) {
        Ok(state) => serial::println(format_args!(
            "[C25OK] K14.C25 GFX12 SCRATCH_REG0 dual multi-bit pattern stability gate: amd_present={} navi48={} profile={} domain={} C24_pattern={} C24_restore={} C24_target={} revalidated={} C24_persisted={} intercycle={} target={:#x} BAR5={} eligible={} cycleA={} restoreA={} cycleB={} restoreB={} dual={} writes={} fingerprint={:#018x} fallback={}",
            state.amd_present,state.navi48,state.profile_verified,state.exact_domain_live,state.c24_pattern_verified,state.c24_restore_verified,state.c24_target_revalidated,state.target_revalidated,state.c24_restore_persisted,state.intercycle_restore_persisted,state.target_dword_offset,state.bar5_ready,state.transaction_eligible,state.cycle_a_pattern_verified,state.cycle_a_restore_verified,state.cycle_b_pattern_verified,state.cycle_b_restore_verified,state.dual_pattern_verified,state.writes_performed,state.transaction_fingerprint,state.fallback_armed
        )),
        Err(error) => { serial::println(format_args!("[FAIL] K14.C25 GFX12 dual multi-bit pattern stability gate failed: {error}")); halt_forever(); }
    }

    match native_gpu_c26::initialize(&mut allocator, boot_info.bootstrap.page_table_root) {
        Ok(state) => serial::println(format_args!(
            "[C26OK] K14.C26 final reviewed GFX12 MMIO allowlist/read-only completion gate: amd_present={} navi48={} profile={} domain={} C25_dual={} C25_target={} REG1={} same_base={} distinct={} adjacent={} allowlist={} REG0={:#x} REG1={:#x} BAR5={} eligible={} attempted={} reads={} valid_reads={} read_proof={} writes={} no_write={} completion={} fingerprint={:#018x} fallback={}",
            state.amd_present,state.navi48,state.profile_verified,state.exact_domain_live,state.c25_dual_pattern_verified,state.c25_target_revalidated,state.reg1_resolved,state.same_gc_base1,state.targets_distinct,state.targets_adjacent,state.allowlist_exact,state.reg0_target_dword_offset,state.reg1_target_dword_offset,state.bar5_ready,state.read_eligible,state.read_attempted,state.read_samples,state.read_samples_valid,state.read_proof_valid,state.writes_performed,state.no_write_verified,state.k14_completion_verified,state.read_fingerprint,state.fallback_armed
        )),
        Err(error) => { serial::println(format_args!("[FAIL] K14.C26 final reviewed GFX12 MMIO allowlist/read-only completion gate failed: {error}")); halt_forever(); }
    }

    match native_gpu_c27::initialize(&mut allocator, boot_info.bootstrap.page_table_root) {
        Ok(state) => serial::println(format_args!(
            "[C27OK] K14.C27 complete Radeon driver core: amd_present={} navi48={} C26={} model={} ownership={} topology={} mmio={} write_reject={} irq_handler={} irq_route={} irq_masked={} reset={} errors={} online={} deferred={} qualified={} fingerprint={:#018x} fallback={}",
            state.amd_present,state.navi48,state.c26_foundation_verified,state.driver_model_verified,state.forge_ownership_verified,state.resource_topology_verified,state.reviewed_mmio_service_verified,state.generic_mmio_write_rejected,state.irq_handler_exercised,state.irq_route_registered,state.irq_masked,state.reset_coordinator_verified,state.error_machine_verified,state.core_online,state.hardware_deferred,state.qualified,state.qualification_fingerprint,state.fallback_armed
        )),
        Err(error) => { serial::println(format_args!("[FAIL] K14.C27 complete Radeon driver core failed: {error}")); halt_forever(); }
    }

    match virtio_blk::initialize_and_verify(&mut allocator) {
        Ok(proof) => serial::println(format_args!(
            "[STOR] VirtIO block DMA read verified at {:02x}:{:02x}.{} queue={} signature={:#06x}",
            proof.probe.function.bus,
            proof.probe.function.device,
            proof.probe.function.function,
            proof.queue_size,
            proof.boot_signature
        )),
        Err(error) => {
            serial::println(format_args!(
                "[STOR] VirtIO proof unavailable: {error}; using loader-resident recovery mirror"
            ));
        }
    }

    if let Err(error) = vfs::mount_boot_volume(boot_info) {
        serial::println(format_args!("[FAIL] K14 VFS mount failed: {error}"));
        halt_forever();
    }
    // C28's cacheable kernel-DMA mappings carry NX PTEs and are exercised
    // immediately, so enable EFER.NXE before their first CPU access. The later
    // user-mapping policy call is intentionally idempotent.
    arch::x86_64::enable_nx();
    match native_gpu_c28::initialize(&mut allocator, boot_info.bootstrap.page_table_root) {
        Ok(state) => serial::println(format_args!(
            "[C28OK] K14.C28 Radeon memory+firmware+recovery: amd_present={} navi48={} C27={} GTT={} reclaim={} persistent={} VRAM={} firmware_parser={} firmware_staging={} firmware_files={} watchdog={} recovery={} DMA={} bus_master_off={} submit={} IRQ_hw={} qualified={} fingerprint={:#018x} fallback={}",
            state.amd_present,state.navi48,state.c27_verified,state.gtt_operational,state.gtt_reclaim_verified,state.persistent_gtt_verified,state.vram_reservation_verified,state.firmware_parser_verified,state.firmware_staging_verified,state.firmware_files_staged,state.watchdog_verified,state.recovery_lifecycle_verified,state.dma_enabled,state.bus_master_off,state.command_submit_enabled,state.physical_irq_enabled,state.qualified,state.qualification_fingerprint,state.fallback_armed
        )),
        Err(error) => { serial::println(format_args!("[FAIL] K14.C28 Radeon memory+firmware+recovery failed: {error}")); halt_forever(); }
    }
    match native_gpu_c29::initialize(&mut allocator) {
        Ok(state) => serial::println(format_args!(
            "[C29OK] K14.C29 Radeon rings+queues+fences+DMA: amd_present={} navi48={} C28={} ring={} queue={} fence={} DMA={} bytes={} exact_SDMA={} GC_base0={} hardware_deferred={} bus_master={} physical_SDMA={} qualified={} fingerprint={:#018x} fallback={}",
            state.amd_present,state.navi48,state.c28_verified,state.ring_ready,state.queue_ready,state.fence_ready,state.dma_ready,state.dma_bytes,state.sdma_register_plan_verified,state.sdma_gc_base0_resolved,state.hardware_deferred,state.bus_master_enabled,state.physical_sdma_programmed,state.qualified,state.qualification_fingerprint,state.fallback_armed
        )),
        Err(error) => { serial::println(format_args!("[FAIL] K14.C29 Radeon rings+queues+fences+DMA failed: {error}")); halt_forever(); }
    }
    match native_gpu_c30::initialize(&mut allocator, boot_info) {
        Ok(state) => serial::println(format_args!(
            "[C30OK] K14.C30 complete basic display engine: amd_present={} C29={} connectors={} active={} mode={}x{} EDID={} scanout={} flips={} modeset={} hotplug={} DCN401={} native_DCN={} qualified={} fingerprint={:#018x} fallback={}",
            state.amd_present,state.c29_verified,state.connector_count,state.active_connector,state.width,state.height,state.edid_verified,state.scanout_verified,state.flips,state.atomic_modeset_verified,state.hotplug_verified,state.dcn401_source_reviewed,state.native_dcn_programmed,state.qualified,state.fingerprint,state.fallback_armed
        )),
        Err(error) => { serial::println(format_args!("[FAIL] K14.C30 complete basic display engine failed: {error}")); halt_forever(); }
    }
    match native_gpu_c31::initialize(&mut allocator, boot_info) {
        Ok(state) => serial::println(format_args!(
            "[C31OK] K14.C31 graphics+compute execution: amd_present={} C30={} shader_upload={} shader_cache={} precache={} command_encoding={} compute_queue={} compute={} elements={} graphics_queue={} graphics={} pixels={} framebuffer={} reference={} physical_GPU={} qualified={} fingerprint={:#018x}",
            state.amd_present,state.c30_verified,state.shader_upload_verified,state.shader_cache_verified,state.precache_entries,state.command_encoding_verified,state.compute_queue_verified,state.compute_dispatch_verified,state.compute_elements,state.graphics_queue_verified,state.graphics_draw_verified,state.triangle_pixels,state.framebuffer_verified,state.reference_execution,state.physical_gpu_execution,state.qualified,state.fingerprint
        )),
        Err(error) => { serial::println(format_args!("[FAIL] K14.C31 graphics+compute execution failed: {error}")); halt_forever(); }
    }
    match native_gpu_c32::initialize(&mut allocator, boot_info) {
        Ok(state) => {
            serial::println(format_args!(
                "[C32OK] K14.C32 production/stability + final K14: C31={} queues={} pressure={} recovery={} IRQ={} concurrency={} display={} multiGPU={} power={} telemetry={} precache={} ABI={} physical_stress={} qualified={} fingerprint={:#018x}",
                state.c31_verified,state.queue_stress_verified,state.memory_pressure_verified,state.hang_recovery_verified,state.interrupt_stress_verified,state.display_compute_concurrency&&state.graphics_compute_concurrency,state.display_stress_verified,state.multi_gpu_enumeration_verified,state.power_policy_verified,state.telemetry_verified,state.shader_precache_frozen,state.userspace_abi_frozen,state.physical_stress_qualified,state.qualified,state.fingerprint
            ));
        },
        Err(error) => { serial::println(format_args!("[FAIL] K14.C32 production/stability final failed: {error}")); halt_forever(); }
    }
    match vfs::log_directory(b"C:\\SYSTEM\\SERVICES") {
        Ok(count) => serial::println(format_args!(
            "[VFS ] C:\\SYSTEM\\SERVICES contains {} entries", count
        )),
        Err(error) => {
            serial::println(format_args!("[FAIL] K14 service directory failed: {error}"));
            halt_forever();
        }
    }
    if let Err(error) = namespace::initialize_core_namespace() {
        serial::println(format_args!("[FAIL] K13 namespace bootstrap failed: {error}"));
        halt_forever();
    }
    if let Err(error) = shared_memory::initialize_core_objects() {
        serial::println(format_args!("[FAIL] K13 shared-memory bootstrap failed: {error}"));
        halt_forever();
    }
    arch::x86_64::enable_nx();
    serial::println(format_args!("[MMU ] NX policy enabled for user mappings"));
    serial::println(format_args!(
        "[PROC] Loading K14 native services from C:\\SYSTEM\\SERVICES"
    ));
    let kernel_return_stack = boot_info.bootstrap.stack_virtual_base + boot_info.bootstrap.stack_size;
    process::launch_user_services(
        &mut allocator,
        boot_info,
        boot_info.bootstrap.page_table_root,
        kernel_return_stack,
        bsp_index,
        timer_initial_count,
    );
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    serial::emergency_println(format_args!(""));
    serial::emergency_println(format_args!("[PANIC] WeaveCore K13 panic"));
    serial::emergency_println(format_args!("[PANIC] {info}"));
    recovery::mark_boot_failed();
    halt_forever();
}
