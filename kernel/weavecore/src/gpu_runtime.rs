//! K13 GPU acceleration orchestration.
//!
//! K13.A qualifies backend-neutral accounting and topology. K13.B brings up a
//! live modern VirtIO-GPU transport. K13.C layers a buffered, damage-tracked,
//! fence-verified compositor presentation path on that qualified transport
//! while retaining the K12 GOP framebuffer as a recovery/fallback scanout.

use crate::{
    display, gpu_fence, gpu_memory, gpu_modeset, gpu_multigpu, gpu_present, gpu_queue,
    gpu_resilience, gpu_topology, memory::FrameAllocator, serial, virtio_gpu,
};
use crate::sync::SpinLock;

pub const FORGEGRAPHICS_ACCEL_ABI_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug)]
pub struct GpuRuntimeState {
    pub adapters: u32,
    pub native_adapters: u32,
    pub virtio_candidates: u32,
    pub transport_ready: bool,
    pub presentation_ready: bool,
    pub scanout_width: u32,
    pub scanout_height: u32,
    pub control_queue_size: u16,
    pub cursor_queue_size: u16,
    pub present_buffers: u32,
    pub presented_frames: u64,
    pub last_present_fence: u64,
    pub resilience_ready: bool,
    pub secondary_candidates: u32,
    pub recovery_cycles: u32,
    pub soak_frames: u32,
}

impl GpuRuntimeState {
    pub const EMPTY: Self = Self {
        adapters: 0,
        native_adapters: 0,
        virtio_candidates: 0,
        transport_ready: false,
        presentation_ready: false,
        scanout_width: 0,
        scanout_height: 0,
        control_queue_size: 0,
        cursor_queue_size: 0,
        present_buffers: 0,
        presented_frames: 0,
        last_present_fence: 0,
        resilience_ready: false,
        secondary_candidates: 0,
        recovery_cycles: 0,
        soak_frames: 0,
    };
}

static STATE: SpinLock<GpuRuntimeState> = SpinLock::new(GpuRuntimeState::EMPTY);

pub fn initialize_foundation() -> Result<GpuRuntimeState, &'static str> {
    gpu_topology::self_test()?;
    let topology = gpu_topology::discover();
    serial::println(format_args!(
        "[GPU ] K13 topology: adapters={} amd={} intel={} nvidia={} virtio={} other={}",
        topology.adapters, topology.amd, topology.intel, topology.nvidia, topology.virtio, topology.other
    ));

    let memory = gpu_memory::run_self_test()?;
    serial::println(format_args!(
        "[VRAM] domain lifecycle self-test: created={} gtt={}MiB vram={}MiB",
        memory.created, memory.final_gtt_bytes >> 20, memory.final_vram_bytes >> 20
    ));

    let queued = gpu_queue::run_self_test()?;
    serial::println(format_args!("[CMDQ] bounded submission self-test: packets={}", queued));

    let (submitted, completed) = gpu_fence::run_self_test()?;
    serial::println(format_args!(
        "[FENC] timeline self-test: submitted={} completed={}", submitted, completed
    ));

    let mode = gpu_modeset::run_self_test()?;
    serial::println(format_args!(
        "[MODE] atomic modeset contract: {}x{} @ {}mHz",
        mode.width, mode.height, mode.refresh_millihz
    ));

    let route = gpu_multigpu::run_self_test()?;
    serial::println(format_args!("[MGPU] transfer policy self-test: route={:?}", route));

    let present_policy = gpu_present::run_self_test()?;
    serial::println(format_args!(
        "[PACE] compositor pacing contract: buffers={} in_flight={} refresh={}mHz period={}ns",
        present_policy.buffers,
        present_policy.max_in_flight,
        present_policy.refresh_millihz,
        present_policy.period_ns
    ));
    serial::println(format_args!(
        "[FBCK] presentation watchdog policy: fallback_after_stalls={} gop_fallback=armed",
        present_policy.fallback_after_stalls
    ));

    let virtio = virtio_gpu::probe();
    if virtio.present {
        serial::println(format_args!(
            "[VGPU] VirtIO-GPU candidate at {:02x}:{:02x}.{} device={:#06x} mmio_bars={} bus_master={} probe_only=true",
            virtio.bus, virtio.device, virtio.function, virtio.device_id, virtio.memory_bars, virtio.bus_master_enabled
        ));
    } else {
        serial::println(format_args!("[VGPU] VirtIO-GPU candidate not present; native/fallback path retained"));
    }

    let state = GpuRuntimeState {
        adapters: topology.adapters as u32,
        native_adapters: topology.native_vendor_count() as u32,
        virtio_candidates: topology.virtio as u32,
        ..GpuRuntimeState::EMPTY
    };
    *STATE.lock() = state;
    serial::println(format_args!(
        "[GACC] ForgeGraphics acceleration ABI v{} foundation passed transport_ready={}",
        FORGEGRAPHICS_ACCEL_ABI_VERSION, state.transport_ready
    ));
    Ok(state)
}

/// Bring up the K13.B modern VirtIO-GPU transport. The K13.A foundation remains
/// independently qualified so a transport failure can be diagnosed without
/// losing the topology/memory/queue/fence contract markers.
pub fn initialize_transport(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
    identity_map_limit: u64,
) -> Result<GpuRuntimeState, &'static str> {
    let foundation = state();
    if foundation.virtio_candidates == 0 {
        serial::println(format_args!(
            "[VPCI] no VirtIO-GPU candidate; accelerated transport deferred to native backend"
        ));
        return Ok(foundation);
    }

    let report = virtio_gpu::initialize_transport(allocator, kernel_cr3, identity_map_limit)?;
    serial::println(format_args!(
        "[VPCI] modern capabilities + VERSION_1 negotiated: driver={} device={} features={:#010x}:{:#010x}",
        report.driver_id,
        report.device_id.0,
        report.negotiated_features_high,
        report.negotiated_features_low
    ));
    serial::println(format_args!(
        "[VQ  ] controlq={} cursorq={} polled bootstrap completion online",
        report.control_queue_size, report.cursor_queue_size
    ));
    serial::println(format_args!(
        "[VDMA] ForgeBus bounded DMA ownership online: framebuffer={} KiB hardware_translation=deferred",
        report.framebuffer_bytes / 1024
    ));
    serial::println(format_args!(
        "[SCAN] VirtIO-GPU resource {} scanout={} {}x{} transfer+flush verified device_scanouts={}",
        1, report.scanout_id, report.width, report.height, report.device_scanouts
    ));

    let state = GpuRuntimeState {
        transport_ready: report.transport_ready,
        scanout_width: report.width,
        scanout_height: report.height,
        control_queue_size: report.control_queue_size,
        cursor_queue_size: report.cursor_queue_size,
        ..foundation
    };
    *STATE.lock() = state;
    serial::println(format_args!(
        "[GACC] ForgeGraphics acceleration ABI v{} transport passed transport_ready={} backend=virtio-gpu-modern",
        FORGEGRAPHICS_ACCEL_ABI_VERSION, state.transport_ready
    ));
    Ok(state)
}

/// K13.C establishes a reusable triple-buffered scanout set on the live K13.B
/// transport and proves dirty-region upload + device-echoed fence completion.
pub fn initialize_presentation(
    allocator: &mut FrameAllocator<'_>,
) -> Result<GpuRuntimeState, &'static str> {
    let prior = state();
    if !prior.transport_ready {
        return Err("K13.C presentation requires a live K13.B GPU transport");
    }
    let report = virtio_gpu::initialize_presentation(allocator)?;
    if report.buffers != gpu_present::PRESENT_BUFFER_COUNT as u32 || report.frames_presented < 3 {
        return Err("K13.C buffered presentation qualification was incomplete");
    }
    serial::println(format_args!(
        "[PRES] triple-buffered compositor scanout online: buffers={} front_resource={} frames={}",
        report.buffers, report.front_resource, report.frames_presented
    ));
    serial::println(format_args!(
        "[DMG ] dirty-region GPU uploads verified: uploads={} partial_rects=true",
        report.damage_uploads
    ));
    serial::println(format_args!(
        "[PFEN] fence-verified presentation complete: last_fence={} echo_verified=true",
        report.last_fence
    ));
    serial::println(format_args!(
        "[FBCK] GOP fallback remains armed after accelerated presentation: {}",
        report.fallback_armed
    ));

    let state = GpuRuntimeState {
        presentation_ready: true,
        present_buffers: report.buffers,
        presented_frames: report.frames_presented,
        last_present_fence: report.last_fence,
        ..prior
    };
    *STATE.lock() = state;
    serial::println(format_args!(
        "[GCOMP] K13.C compositor presentation ready: buffers={} frames={} fence={} backend=virtio-gpu-2d",
        state.present_buffers, state.presented_frames, state.last_present_fence
    ));
    Ok(state)
}

/// K13.D combines backend-neutral resilience policy tests with a live
/// presentation soak and a controlled suspend/fallback/rearm cycle on the
/// already-qualified K13.B/K13.C VirtIO transport.
pub fn initialize_resilience_qualification() -> Result<GpuRuntimeState, &'static str> {
    const SOAK_FRAMES: u32 = 64;

    let prior = state();
    if !prior.presentation_ready || !virtio_gpu::presentation_ready() {
        return Err("K13.D resilience qualification requires K13.C presentation");
    }

    let policy = gpu_resilience::run_self_test()?;
    serial::println(format_args!(
        "[RSLN] GPU health/rebind state machine: threshold={} recoveries={} failovers={} fallback={}",
        policy.recovery_threshold,
        policy.recoveries,
        policy.failovers,
        policy.fallback_armed
    ));
    serial::println(format_args!(
        "[HOTG] PCIe GPU hotplug policy self-test: events={} generation_safe=true",
        policy.hotplug_events
    ));
    serial::println(format_args!(
        "[MOUT] multi-scanout policy self-test: managed={} promoted_primary={} generation_safe=true",
        policy.managed_scanouts,
        policy.promoted_scanout
    ));

    let topology = gpu_topology::discover();
    let secondary_candidates = topology.adapters.saturating_sub(1) as u32;
    serial::println(format_args!(
        "[MGP2] multi-GPU presentation policy: adapters={} secondary={} route={:?} standby_safe=true",
        topology.adapters,
        secondary_candidates,
        policy.transfer_route
    ));

    let mut last_fence = prior.last_present_fence;
    let mut last_frame = prior.presented_frames;
    for frame in 0..SOAK_FRAMES {
        let result = virtio_gpu::present_compositor_frame(0x13d0 + u64::from(frame))?;
        if result.fence_id <= last_fence || result.frame_sequence <= last_frame {
            return Err("K13.D presentation soak observed non-monotonic completion");
        }
        last_fence = result.fence_id;
        last_frame = result.frame_sequence;
    }
    serial::println(format_args!(
        "[SOAK] presentation stress: frames={} last_frame={} last_fence={} monotonic=true",
        SOAK_FRAMES,
        last_frame,
        last_fence
    ));

    let suspended = virtio_gpu::suspend_presentation_for_recovery()?;
    if !suspended.bus_master_enabled || !suspended.driver_ok {
        return Err("K13.D recovery qualification found unhealthy live transport");
    }
    if virtio_gpu::present_compositor_frame(0xdead).is_ok() {
        return Err("K13.D recovery fence failed to block presentation");
    }
    if !display::state().firmware_fallback {
        return Err("K13.D recovery requires the K12 GOP fallback to remain armed");
    }
    serial::println(format_args!(
        "[DLOS] controlled device-loss fence: device={} frame={} fence={} GOP_fallback=true blocked=true",
        suspended.device_id.0,
        suspended.frame_sequence,
        suspended.completed_fence
    ));

    let resumed = virtio_gpu::resume_presentation_after_recovery()?;
    if !resumed.bus_master_enabled || !resumed.driver_ok {
        return Err("K13.D transport rearm did not restore a healthy backend");
    }
    let recovered = virtio_gpu::present_compositor_frame(0x13d)?;
    serial::println(format_args!(
        "[REBD] transport rearm verified: device={} frame={} fence={} bus_master=true driver_ok=true",
        resumed.device_id.0,
        recovered.frame_sequence,
        recovered.fence_id
    ));

    let next = GpuRuntimeState {
        resilience_ready: true,
        secondary_candidates,
        recovery_cycles: prior.recovery_cycles.saturating_add(1),
        soak_frames: SOAK_FRAMES,
        presented_frames: recovered.frame_sequence,
        last_present_fence: recovered.fence_id,
        ..prior
    };
    *STATE.lock() = next;
    Ok(next)
}

/// DISPLAYD capability-mediated recovery request. K13.D deliberately performs
/// a controlled presentation fence/rearm here; physical PCI FLR and slot power
/// cycling remain later native-backend work and are not claimed by this API.
pub fn recover_from_displayd(pattern_seed: u64) -> Result<u64, &'static str> {
    let prior = state();
    if !prior.resilience_ready {
        return Err("K13.D resilience path is not ready");
    }

    let suspended = virtio_gpu::suspend_presentation_for_recovery()?;
    if virtio_gpu::present_compositor_frame(pattern_seed).is_ok() {
        let _ = virtio_gpu::resume_presentation_after_recovery();
        return Err("DISPLAYD recovery did not fence presentation");
    }
    if !display::state().firmware_fallback {
        let _ = virtio_gpu::resume_presentation_after_recovery();
        return Err("DISPLAYD recovery lost firmware fallback");
    }
    let resumed = match virtio_gpu::resume_presentation_after_recovery() {
        Ok(report) => report,
        Err(error) => {
            virtio_gpu::disable_accelerated_presentation();
            *STATE.lock() = GpuRuntimeState {
                transport_ready: false,
                presentation_ready: false,
                resilience_ready: false,
                ..prior
            };
            return Err(error);
        }
    };
    if !resumed.driver_ok || !resumed.bus_master_enabled {
        return Err("DISPLAYD recovery found transport unhealthy after rearm");
    }
    let result = match virtio_gpu::present_compositor_frame(pattern_seed.saturating_add(1)) {
        Ok(result) => result,
        Err(error) => {
            virtio_gpu::disable_accelerated_presentation();
            *STATE.lock() = GpuRuntimeState {
                transport_ready: false,
                presentation_ready: false,
                resilience_ready: false,
                ..prior
            };
            return Err(error);
        }
    };

    *STATE.lock() = GpuRuntimeState {
        recovery_cycles: prior.recovery_cycles.saturating_add(1),
        presented_frames: result.frame_sequence,
        last_present_fence: result.fence_id,
        ..prior
    };
    serial::println(format_args!(
        "[URCV] DISPLAYD recovery: device={} frame={} fence={} fallback_verified=true",
        suspended.device_id.0,
        result.frame_sequence,
        result.fence_id
    ));
    Ok(result.fence_id)
}

/// Capability-mediated present entry used by DISPLAYD. The caller check lives
/// in the syscall layer; this function only advances the qualified backend.
pub fn present_from_displayd(pattern_seed: u64) -> Result<u64, &'static str> {
    let prior = state();
    if !prior.presentation_ready || !virtio_gpu::presentation_ready() {
        return Err("K13.C presentation path is not ready");
    }
    let result = match virtio_gpu::present_compositor_frame(pattern_seed) {
        Ok(result) => result,
        Err(error) => {
            virtio_gpu::disable_accelerated_presentation();
            let fallback_state = GpuRuntimeState {
                transport_ready: false,
                presentation_ready: false,
                resilience_ready: false,
                ..prior
            };
            *STATE.lock() = fallback_state;
            serial::println(format_args!(
                "[FBCK] accelerated GPU path fenced after presentation failure; GOP recovery scanout retained"
            ));
            return Err(error);
        }
    };
    let state = GpuRuntimeState {
        presented_frames: result.frame_sequence,
        last_present_fence: result.fence_id,
        ..prior
    };
    *STATE.lock() = state;
    serial::println(format_args!(
        "[UPRS] DISPLAYD present: frame={} resource={} fence={} damage={},{} {}x{}",
        result.frame_sequence,
        result.resource_id,
        result.fence_id,
        result.damage.x,
        result.damage.y,
        result.damage.width,
        result.damage.height
    ));
    Ok(result.fence_id)
}

#[must_use]
pub fn state() -> GpuRuntimeState { *STATE.lock() }

/// Packed userspace status: bits 0..15 adapter count, bit 16 VirtIO candidate,
/// bit 17 native-vendor adapter, bit 29 K13.D resilience, bit 30 buffered
/// presentation, bit 31 live command transport. Bits 32..47 and 48..63 expose
/// scanout dimensions.
#[must_use]
pub fn packed_status() -> u64 {
    let state = state();
    let mut value = (state.adapters as u64) & 0xffff;
    if state.virtio_candidates != 0 { value |= 1 << 16; }
    if state.native_adapters != 0 { value |= 1 << 17; }
    if state.resilience_ready { value |= 1 << 29; }
    if state.presentation_ready { value |= 1 << 30; }
    if state.transport_ready { value |= 1 << 31; }
    value |= (u64::from(state.scanout_width) & 0xffff) << 32;
    value |= (u64::from(state.scanout_height) & 0xffff) << 48;
    value
}
