//! K15.3 ForgeAudio cyclic DMA transport.
//!
//! This layer owns real physically-contiguous DMA-capable memory, period
//! geometry, producer/device ownership transitions, cumulative position and
//! XRUN accounting. Hardware arming is fail-closed: a backend must present a
//! translated IOMMU lease covering the exact ring before production DMA calls
//! are accepted. K15.4's HDA backend is the first audio hardware backend that
//! will create such a lease. The K15.3 QEMU self-test validates the complete
//! transport core without fabricating an audio device or fake IRQ/DMA event.

use core::ptr;

use crate::{
    memory::{FrameAllocator, FRAME_SIZE},
    paging, serial, translated_dma,
};
use titanweave_forgeaudio_abi::AudioDirection;

pub const FORGEAUDIO_DMA_TRANSPORT_VERSION: u32 = 1;
pub const MAX_AUDIO_DMA_PERIODS: usize = 32;
pub const MAX_AUDIO_DMA_RING_BYTES: u64 = 1024 * 1024;
pub const MIN_AUDIO_DMA_PERIOD_BYTES: u64 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodOwnership {
    Free,
    CpuWritable,
    QueuedToDevice,
    DeviceReady,
    DeviceOwned,
    CpuReadable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioDmaAccess {
    DeviceRead,
    DeviceWrite,
}

#[derive(Clone, Copy, Debug)]
pub struct DmaIsolationLease {
    pub requester: u16,
    pub domain_id: u16,
    pub iova_base: u64,
    pub mapped_bytes: u64,
    pub physical_base: u64,
    pub physical_bytes: u64,
    pub access: AudioDmaAccess,
    pub hardware_translated: bool,
    pub generation: u32,
}

impl DmaIsolationLease {
    pub fn new_translated(
        requester: u16,
        domain_id: u16,
        iova_base: u64,
        mapped_bytes: u64,
        physical_base: u64,
        physical_bytes: u64,
        access: AudioDmaAccess,
        generation: u32,
    ) -> Result<Self, &'static str> {
        if requester == 0 || domain_id == 0 || iova_base == 0 || generation == 0 {
            return Err("audio DMA translated lease identity is incomplete");
        }
        if mapped_bytes == 0 || physical_bytes == 0 || mapped_bytes < physical_bytes {
            return Err("audio DMA translated lease range is invalid");
        }
        Ok(Self {
            requester,
            domain_id,
            iova_base,
            mapped_bytes,
            physical_base,
            physical_bytes,
            access,
            hardware_translated: true,
            generation,
        })
    }

    fn validate_for(&self, buffer: &AudioDmaBuffer, direction: AudioDirection) -> Result<(), &'static str> {
        if !self.hardware_translated {
            return Err("audio DMA hardware arm requires translated IOMMU isolation");
        }
        if self.requester == 0 || self.domain_id == 0 || self.iova_base == 0 || self.generation == 0 {
            return Err("audio DMA translated lease identity is incomplete");
        }
        if self.physical_base != buffer.physical_address || self.physical_bytes < buffer.mapped_bytes {
            return Err("audio DMA translated lease does not cover ring physical memory");
        }
        if self.mapped_bytes < buffer.mapped_bytes {
            return Err("audio DMA translated lease IOVA aperture is too small");
        }
        let expected = match direction {
            AudioDirection::Playback => AudioDmaAccess::DeviceRead,
            AudioDirection::Capture => AudioDmaAccess::DeviceWrite,
            AudioDirection::Duplex => return Err("one DMA transport cannot represent duplex direction"),
        };
        if self.access != expected {
            return Err("audio DMA translated lease permissions do not match stream direction");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PeriodDescriptor {
    pub index: u16,
    pub ownership: PeriodOwnership,
    pub physical_address: u64,
    pub virtual_address: u64,
    pub byte_offset: u32,
    pub byte_length: u32,
    pub frame_capacity: u32,
    pub committed_frames: u32,
    pub sequence: u64,
}

impl PeriodDescriptor {
    const EMPTY: Self = Self {
        index: 0,
        ownership: PeriodOwnership::Free,
        physical_address: 0,
        virtual_address: 0,
        byte_offset: 0,
        byte_length: 0,
        frame_capacity: 0,
        committed_frames: 0,
        sequence: 0,
    };
}

#[derive(Clone, Copy, Debug)]
pub struct BackendPeriod {
    pub index: usize,
    pub device_address: u64,
    pub physical_address: u64,
    pub virtual_address: u64,
    pub byte_length: u32,
    pub frames: u32,
    pub sequence: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AudioDmaSnapshot {
    pub frame_position: u64,
    pub byte_position: u64,
    pub completed_periods: u64,
    pub next_period: u32,
    pub wrap_count: u64,
    pub underruns: u64,
    pub overruns: u64,
    pub hardware_armed: bool,
    pub isolation_generation: u32,
}

struct AudioDmaBuffer {
    physical_address: u64,
    virtual_address: u64,
    requested_bytes: u64,
    mapped_bytes: u64,
    pages: u64,
    released: bool,
}

impl AudioDmaBuffer {
    fn allocate(
        allocator: &mut FrameAllocator<'_>,
        kernel_cr3: u64,
        requested_bytes: u64,
    ) -> Result<Self, &'static str> {
        if requested_bytes == 0 || requested_bytes > MAX_AUDIO_DMA_RING_BYTES {
            return Err("audio DMA ring size is outside bounded transport limits");
        }
        let pages = requested_bytes
            .checked_add(FRAME_SIZE - 1)
            .ok_or("audio DMA ring page rounding overflow")?
            / FRAME_SIZE;
        let mapped_bytes = pages
            .checked_mul(FRAME_SIZE)
            .ok_or("audio DMA mapped byte count overflow")?;
        let physical_address = allocator
            .allocate_contiguous(pages)
            .ok_or("audio DMA contiguous physical allocation failed")?;
        let virtual_address = match paging::map_kernel_dma(
            allocator,
            kernel_cr3,
            physical_address,
            mapped_bytes,
        ) {
            Ok(address) => address,
            Err(error) => {
                let _ = allocator.deallocate_contiguous(physical_address, pages);
                return Err(error);
            }
        };
        let zero_bytes = usize::try_from(mapped_bytes).map_err(|_| "audio DMA ring does not fit kernel pointer size")?;
        unsafe { ptr::write_bytes(virtual_address as *mut u8, 0, zero_bytes) };
        Ok(Self {
            physical_address,
            virtual_address,
            requested_bytes,
            mapped_bytes,
            pages,
            released: false,
        })
    }

    fn release(
        &mut self,
        allocator: &mut FrameAllocator<'_>,
        kernel_cr3: u64,
    ) -> Result<(), &'static str> {
        if self.released {
            return Err("audio DMA ring already released");
        }
        let zero_bytes = usize::try_from(self.mapped_bytes).map_err(|_| "audio DMA ring does not fit kernel pointer size")?;
        unsafe { ptr::write_bytes(self.virtual_address as *mut u8, 0, zero_bytes) };
        paging::unmap_kernel_dma(kernel_cr3, self.virtual_address, self.mapped_bytes)?;
        allocator.deallocate_contiguous(self.physical_address, self.pages)?;
        self.released = true;
        self.physical_address = 0;
        self.virtual_address = 0;
        self.requested_bytes = 0;
        self.mapped_bytes = 0;
        self.pages = 0;
        Ok(())
    }
}

pub struct AudioDmaTransport {
    direction: AudioDirection,
    frame_stride_bytes: u32,
    period_frames: u32,
    period_bytes: u32,
    period_count: usize,
    ring: AudioDmaBuffer,
    periods: [PeriodDescriptor; MAX_AUDIO_DMA_PERIODS],
    next_device_period: usize,
    inflight_period: Option<usize>,
    frame_position: u64,
    completed_periods: u64,
    wrap_count: u64,
    underruns: u64,
    overruns: u64,
    hardware_armed: bool,
    device_iova_base: u64,
    isolation_generation: u32,
}

impl AudioDmaTransport {
    pub fn allocate(
        allocator: &mut FrameAllocator<'_>,
        kernel_cr3: u64,
        direction: AudioDirection,
        frame_stride_bytes: u32,
        period_frames: u32,
        period_count: usize,
    ) -> Result<Self, &'static str> {
        if !matches!(direction, AudioDirection::Playback | AudioDirection::Capture) {
            return Err("DMA transport requires one playback or capture direction");
        }
        if frame_stride_bytes == 0 || period_frames == 0 {
            return Err("audio DMA frame geometry is empty");
        }
        if period_count < 2 || period_count > MAX_AUDIO_DMA_PERIODS {
            return Err("audio DMA period count is outside bounded transport limits");
        }
        let period_bytes_u64 = u64::from(frame_stride_bytes)
            .checked_mul(u64::from(period_frames))
            .ok_or("audio DMA period byte count overflow")?;
        if period_bytes_u64 < MIN_AUDIO_DMA_PERIOD_BYTES || period_bytes_u64 > u64::from(u32::MAX) {
            return Err("audio DMA period byte count is outside transport bounds");
        }
        let ring_bytes = period_bytes_u64
            .checked_mul(period_count as u64)
            .ok_or("audio DMA ring byte count overflow")?;
        if ring_bytes > MAX_AUDIO_DMA_RING_BYTES {
            return Err("audio DMA cyclic ring exceeds bounded transport size");
        }
        let ring = AudioDmaBuffer::allocate(allocator, kernel_cr3, ring_bytes)?;
        let period_bytes = period_bytes_u64 as u32;
        let initial_ownership = match direction {
            AudioDirection::Playback => PeriodOwnership::CpuWritable,
            AudioDirection::Capture => PeriodOwnership::DeviceReady,
            AudioDirection::Duplex => unreachable!(),
        };
        let mut periods = [PeriodDescriptor::EMPTY; MAX_AUDIO_DMA_PERIODS];
        for (index, slot) in periods.iter_mut().take(period_count).enumerate() {
            let byte_offset = (index as u64)
                .checked_mul(period_bytes_u64)
                .ok_or("audio DMA period offset overflow")?;
            *slot = PeriodDescriptor {
                index: index as u16,
                ownership: initial_ownership,
                physical_address: ring.physical_address + byte_offset,
                virtual_address: ring.virtual_address + byte_offset,
                byte_offset: byte_offset as u32,
                byte_length: period_bytes,
                frame_capacity: period_frames,
                committed_frames: 0,
                sequence: 0,
            };
        }
        Ok(Self {
            direction,
            frame_stride_bytes,
            period_frames,
            period_bytes,
            period_count,
            ring,
            periods,
            next_device_period: 0,
            inflight_period: None,
            frame_position: 0,
            completed_periods: 0,
            wrap_count: 0,
            underruns: 0,
            overruns: 0,
            hardware_armed: false,
            device_iova_base: 0,
            isolation_generation: 0,
        })
    }

    #[must_use]
    pub fn period_count(&self) -> usize { self.period_count }

    #[must_use]
    pub fn period_descriptor(&self, index: usize) -> Option<PeriodDescriptor> {
        if index >= self.period_count { None } else { Some(self.periods[index]) }
    }

    pub fn queue_playback_period(&mut self, index: usize, frames: u32) -> Result<(), &'static str> {
        if self.direction != AudioDirection::Playback {
            return Err("capture DMA transport cannot queue playback data");
        }
        if index >= self.period_count || frames == 0 || frames > self.period_frames {
            return Err("playback DMA period commit is invalid");
        }
        let period = &mut self.periods[index];
        if period.ownership != PeriodOwnership::CpuWritable {
            return Err("playback DMA period is not CPU-writable");
        }
        period.committed_frames = frames;
        period.sequence = period.sequence.wrapping_add(1).max(1);
        period.ownership = PeriodOwnership::QueuedToDevice;
        Ok(())
    }

    pub fn release_capture_period(&mut self, index: usize) -> Result<(), &'static str> {
        if self.direction != AudioDirection::Capture {
            return Err("playback DMA transport has no capture period to release");
        }
        if index >= self.period_count {
            return Err("capture DMA period index is outside ring");
        }
        let period = &mut self.periods[index];
        if period.ownership != PeriodOwnership::CpuReadable {
            return Err("capture DMA period is not CPU-readable");
        }
        period.committed_frames = 0;
        period.ownership = PeriodOwnership::DeviceReady;
        Ok(())
    }

    pub fn arm_hardware(&mut self, lease: &DmaIsolationLease) -> Result<(), &'static str> {
        if self.hardware_armed {
            return Err("audio DMA transport is already hardware-armed");
        }
        if self.inflight_period.is_some() {
            return Err("audio DMA transport cannot arm with a period in flight");
        }
        lease.validate_for(&self.ring, self.direction)?;
        self.hardware_armed = true;
        self.device_iova_base = lease.iova_base;
        self.isolation_generation = lease.generation;
        Ok(())
    }

    pub fn disarm_hardware(&mut self) -> Result<(), &'static str> {
        if self.inflight_period.is_some() {
            return Err("audio DMA transport cannot disarm with a period in flight");
        }
        self.hardware_armed = false;
        self.device_iova_base = 0;
        self.isolation_generation = 0;
        Ok(())
    }

    pub fn backend_acquire_next(&mut self) -> Result<BackendPeriod, &'static str> {
        if !self.hardware_armed {
            return Err("audio DMA backend cannot acquire an unarmed transport");
        }
        self.acquire_next_core()
    }

    pub fn backend_complete_period(&mut self, index: usize, frames: u32) -> Result<(), &'static str> {
        if !self.hardware_armed {
            return Err("audio DMA backend cannot complete an unarmed transport");
        }
        self.complete_period_core(index, frames)
    }

    fn acquire_next_core(&mut self) -> Result<BackendPeriod, &'static str> {
        if self.inflight_period.is_some() {
            return Err("audio DMA backend already owns an in-flight period");
        }
        let index = self.next_device_period;
        let period = &mut self.periods[index];
        let ready = match self.direction {
            AudioDirection::Playback => period.ownership == PeriodOwnership::QueuedToDevice,
            AudioDirection::Capture => period.ownership == PeriodOwnership::DeviceReady,
            AudioDirection::Duplex => false,
        };
        if !ready {
            match self.direction {
                AudioDirection::Playback => self.underruns = self.underruns.saturating_add(1),
                AudioDirection::Capture => self.overruns = self.overruns.saturating_add(1),
                AudioDirection::Duplex => {}
            }
            return Err(match self.direction {
                AudioDirection::Playback => "ForgeAudio playback DMA underrun",
                AudioDirection::Capture => "ForgeAudio capture DMA overrun",
                AudioDirection::Duplex => "invalid duplex DMA transport",
            });
        }
        let frames = match self.direction {
            AudioDirection::Playback => period.committed_frames,
            AudioDirection::Capture => period.frame_capacity,
            AudioDirection::Duplex => 0,
        };
        if frames == 0 {
            self.underruns = self.underruns.saturating_add(1);
            return Err("ForgeAudio playback DMA period has zero committed frames");
        }
        period.ownership = PeriodOwnership::DeviceOwned;
        self.inflight_period = Some(index);
        let device_address = if self.hardware_armed {
            self.device_iova_base
                .checked_add(u64::from(period.byte_offset))
                .ok_or("audio DMA device IOVA period address overflow")?
        } else {
            0
        };
        Ok(BackendPeriod {
            index,
            device_address,
            physical_address: period.physical_address,
            virtual_address: period.virtual_address,
            byte_length: period.byte_length,
            frames,
            sequence: period.sequence,
        })
    }

    fn complete_period_core(&mut self, index: usize, frames: u32) -> Result<(), &'static str> {
        if self.inflight_period != Some(index) || index >= self.period_count {
            return Err("audio DMA completion does not match the in-flight period");
        }
        if frames == 0 || frames > self.period_frames {
            return Err("audio DMA completion frame count is invalid");
        }
        let period = &mut self.periods[index];
        if period.ownership != PeriodOwnership::DeviceOwned {
            return Err("audio DMA completion arrived for a period not owned by device");
        }
        match self.direction {
            AudioDirection::Playback => {
                if frames > period.committed_frames {
                    return Err("audio DMA playback completion exceeds committed frames");
                }
                period.committed_frames = 0;
                period.ownership = PeriodOwnership::CpuWritable;
            }
            AudioDirection::Capture => {
                period.committed_frames = frames;
                period.sequence = period.sequence.wrapping_add(1).max(1);
                period.ownership = PeriodOwnership::CpuReadable;
            }
            AudioDirection::Duplex => return Err("invalid duplex DMA transport"),
        }
        self.frame_position = self
            .frame_position
            .checked_add(u64::from(frames))
            .ok_or("audio DMA frame position overflow")?;
        self.completed_periods = self.completed_periods.saturating_add(1);
        self.inflight_period = None;
        self.next_device_period += 1;
        if self.next_device_period == self.period_count {
            self.next_device_period = 0;
            self.wrap_count = self.wrap_count.saturating_add(1);
        }
        Ok(())
    }

    #[must_use]
    pub fn snapshot(&self) -> AudioDmaSnapshot {
        AudioDmaSnapshot {
            frame_position: self.frame_position,
            byte_position: (self.next_device_period as u64) * u64::from(self.period_bytes),
            completed_periods: self.completed_periods,
            next_period: self.next_device_period as u32,
            wrap_count: self.wrap_count,
            underruns: self.underruns,
            overruns: self.overruns,
            hardware_armed: self.hardware_armed,
            isolation_generation: self.isolation_generation,
        }
    }

    pub fn release(mut self, allocator: &mut FrameAllocator<'_>, kernel_cr3: u64) -> Result<(), &'static str> {
        if self.hardware_armed || self.inflight_period.is_some() {
            return Err("audio DMA transport must be quiescent before release");
        }
        self.ring.release(allocator, kernel_cr3)
    }

    fn selftest_acquire_next(&mut self) -> Result<BackendPeriod, &'static str> {
        self.acquire_next_core()
    }

    fn selftest_complete_period(&mut self, index: usize, frames: u32) -> Result<(), &'static str> {
        self.complete_period_core(index, frames)
    }

    fn write_period_pattern(&self, index: usize, seed: u8) -> Result<(), &'static str> {
        if index >= self.period_count {
            return Err("audio DMA pattern period outside ring");
        }
        let period = self.periods[index];
        for offset in 0..period.byte_length as usize {
            unsafe {
                ptr::write_volatile(
                    (period.virtual_address as *mut u8).add(offset),
                    seed.wrapping_add((offset as u8).wrapping_mul(17)),
                )
            };
        }
        Ok(())
    }

    fn verify_period_pattern(&self, index: usize, seed: u8) -> Result<(), &'static str> {
        if index >= self.period_count {
            return Err("audio DMA verify period outside ring");
        }
        let period = self.periods[index];
        for offset in 0..period.byte_length as usize {
            let expected = seed.wrapping_add((offset as u8).wrapping_mul(17));
            let actual = unsafe { ptr::read_volatile((period.virtual_address as *const u8).add(offset)) };
            if actual != expected {
                return Err("audio DMA ring memory pattern mismatch");
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DmaQualificationReport {
    pub version: u32,
    pub real_dma_memory: bool,
    pub cyclic_periods: bool,
    pub period_completion: bool,
    pub position_tracking: bool,
    pub ownership_enforced: bool,
    pub iommu_fail_closed: bool,
    pub translated_platform_qualified: bool,
    pub playback_underruns: u64,
    pub capture_overruns: u64,
    pub playback_wraps: u64,
    pub completed_playback_periods: u64,
    pub hardware_audio_deferred: bool,
}

pub fn run_self_test(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
) -> Result<DmaQualificationReport, &'static str> {
    const STRIDE: u32 = 4;
    const FRAMES: u32 = 128;
    const PERIODS: usize = 4;

    serial::println(format_args!(
        "[K15DMA] ForgeAudio DMA transport qualification start: version={} cyclic=true bounded=true fake_dma=false",
        FORGEAUDIO_DMA_TRANSPORT_VERSION
    ));

    let translated_platform_qualified = translated_dma::hardware_translation_qualified();
    let translation = translated_dma::state();

    let mut playback = AudioDmaTransport::allocate(
        allocator,
        kernel_cr3,
        AudioDirection::Playback,
        STRIDE,
        FRAMES,
        PERIODS,
    )?;
    if playback.ring.physical_address == 0
        || playback.ring.virtual_address == 0
        || playback.ring.requested_bytes != u64::from(STRIDE) * u64::from(FRAMES) * PERIODS as u64
    {
        return Err("ForgeAudio DMA self-test did not allocate a real bounded ring");
    }
    serial::println(format_args!(
        "[K15DMA] real cyclic ring: phys={:#x} virt={:#x} requested={} mapped={} period_bytes={} periods={} frames_per_period={}",
        playback.ring.physical_address,
        playback.ring.virtual_address,
        playback.ring.requested_bytes,
        playback.ring.mapped_bytes,
        playback.period_bytes,
        playback.period_count,
        playback.period_frames,
    ));

    // Hardware arming must remain impossible without an exact translated lease.
    let invalid_lease = DmaIsolationLease {
        requester: translation.requester,
        domain_id: translation.domain_id,
        iova_base: 0,
        mapped_bytes: playback.ring.mapped_bytes,
        physical_base: playback.ring.physical_address,
        physical_bytes: playback.ring.mapped_bytes,
        access: AudioDmaAccess::DeviceRead,
        hardware_translated: false,
        generation: 1,
    };
    let raw_arm_rejected = playback.arm_hardware(&invalid_lease).is_err() && !playback.hardware_armed;
    if !raw_arm_rejected {
        return Err("ForgeAudio DMA transport accepted an untranslated hardware lease");
    }
    serial::println(format_args!(
        "[K15DMA] isolation gate: platform_translated={} backend={:?} requester={:#06x} domain={} raw_arm_rejected={} audio_hw_deferred=true fake_dma=false",
        translated_platform_qualified,
        translation.backend,
        translation.requester,
        translation.domain_id,
        raw_arm_rejected,
    ));

    // Contract-level period accounting test. No audio device or IRQ is invented:
    // self-test invokes the same ownership/completion state machine directly,
    // while production backend entry points remain hardware-arm protected.
    for index in 0..PERIODS {
        playback.write_period_pattern(index, (0x31u8).wrapping_add(index as u8))?;
        playback.verify_period_pattern(index, (0x31u8).wrapping_add(index as u8))?;
        playback.queue_playback_period(index, FRAMES)?;
    }
    for cycle in 0..8usize {
        let period = playback.selftest_acquire_next()?;
        playback.verify_period_pattern(period.index, (0x31u8).wrapping_add(period.index as u8))?;
        playback.selftest_complete_period(period.index, FRAMES)?;
        playback.write_period_pattern(period.index, (0x31u8).wrapping_add(period.index as u8))?;
        playback.queue_playback_period(period.index, FRAMES)?;
        if cycle == 7 && playback.snapshot().wrap_count != 2 {
            return Err("ForgeAudio DMA cyclic wrap accounting is incorrect");
        }
    }
    let before_underrun = playback.snapshot();
    let starved = playback.selftest_acquire_next()?;
    playback.selftest_complete_period(starved.index, FRAMES)?;
    // Leave the completed period CPU-owned, walk the remaining ring once, and
    // require the next wrap to detect the missing playback period.
    for _ in 0..(PERIODS - 1) {
        let period = playback.selftest_acquire_next()?;
        playback.selftest_complete_period(period.index, FRAMES)?;
        playback.write_period_pattern(period.index, (0x31u8).wrapping_add(period.index as u8))?;
        playback.queue_playback_period(period.index, FRAMES)?;
    }
    if playback.selftest_acquire_next().is_ok() {
        return Err("ForgeAudio DMA playback underrun was not detected");
    }
    playback.write_period_pattern(starved.index, (0x31u8).wrapping_add(starved.index as u8))?;
    playback.queue_playback_period(starved.index, FRAMES)?;
    let after_underrun = playback.snapshot();
    if after_underrun.underruns != before_underrun.underruns + 1 {
        return Err("ForgeAudio DMA playback underrun counter did not advance exactly once");
    }

    let mut capture = AudioDmaTransport::allocate(
        allocator,
        kernel_cr3,
        AudioDirection::Capture,
        STRIDE,
        FRAMES,
        PERIODS,
    )?;
    for _ in 0..PERIODS {
        let period = capture.selftest_acquire_next()?;
        capture.selftest_complete_period(period.index, FRAMES)?;
    }
    if capture.selftest_acquire_next().is_ok() {
        return Err("ForgeAudio DMA capture overrun was not detected");
    }
    let capture_snapshot = capture.snapshot();
    if capture_snapshot.overruns != 1 {
        return Err("ForgeAudio DMA capture overrun counter did not advance exactly once");
    }
    for index in 0..PERIODS {
        capture.release_capture_period(index)?;
    }

    let playback_snapshot = playback.snapshot();
    serial::println(format_args!(
        "[K15DMA] cyclic accounting: completed={} wraps={} frames={} byte_pos={} ownership=true completion_source=transport_core_selftest",
        playback_snapshot.completed_periods,
        playback_snapshot.wrap_count,
        playback_snapshot.frame_position,
        playback_snapshot.byte_position,
    ));
    serial::println(format_args!(
        "[K15DMA] XRUN detection: playback_underruns={} capture_overruns={} bounded=true",
        playback_snapshot.underruns,
        capture_snapshot.overruns,
    ));

    capture.release(allocator, kernel_cr3)?;
    playback.release(allocator, kernel_cr3)?;

    let report = DmaQualificationReport {
        version: FORGEAUDIO_DMA_TRANSPORT_VERSION,
        real_dma_memory: true,
        cyclic_periods: true,
        period_completion: true,
        position_tracking: true,
        ownership_enforced: true,
        iommu_fail_closed: raw_arm_rejected,
        translated_platform_qualified,
        playback_underruns: playback_snapshot.underruns,
        capture_overruns: capture_snapshot.overruns,
        playback_wraps: playback_snapshot.wrap_count,
        completed_playback_periods: playback_snapshot.completed_periods,
        hardware_audio_deferred: true,
    };
    serial::println(format_args!(
        "[K15OK] K15.3 ForgeAudio audio DMA transport qualified: cyclic={} period_completion={} position={} ownership={} iommu_fail_closed={} translated_platform={} xrun=true hardware_audio=false fake_dma=false",
        report.cyclic_periods,
        report.period_completion,
        report.position_tracking,
        report.ownership_enforced,
        report.iommu_fail_closed,
        report.translated_platform_qualified,
    ));
    Ok(report)
}
