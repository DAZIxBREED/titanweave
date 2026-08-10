//! K15.4 ForgeAudio real High Definition Audio hardware backend.
//!
//! This gate drives an actual PCI HDA controller model through its MMIO
//! register interface.  CORB/RIRB and stream BDLs are DMA-visible only inside
//! an exact translated-IOMMU window.  MSI is delivered through Titanweave's
//! device interrupt router.  The qualification path performs real controller
//! reset, codec/widget discovery, playback DMA and capture DMA; it does not
//! synthesize device completion or invent a placeholder audio endpoint.

use core::{
    arch::asm,
    ptr,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    arch::x86_64,
    device::DeviceId,
    forgeaudio,
    forgeaudio_dma::{AudioDmaAccess, AudioDmaTransport, DmaIsolationLease},
    forgebus,
    kernel_runtime,
    memory::{FrameAllocator, FRAME_SIZE},
    msi,
    paging,
    pci::{self, PciFunction},
    pci_address::PciAddress,
    percpu,
    serial,
    sync::SpinLock,
    translated_dma::{self, TemporaryDmaRegion},
};
use titanweave_forgeaudio_abi::{
    AudioDeviceInfo, AudioDirection, AudioEndpointInfo, AudioSampleFormat,
    AUDIO_BACKEND_HDA, AUDIO_DEVICE_FLAG_CAPTURE, AUDIO_DEVICE_FLAG_CLOCK_MASTER,
    AUDIO_DEVICE_FLAG_FULL_DUPLEX, AUDIO_DEVICE_FLAG_PLAYBACK,
    AUDIO_ENDPOINT_FLAG_DEFAULT, AUDIO_ENDPOINT_FLAG_LINE_LEVEL,
    AUDIO_ENDPOINT_FLAG_MICROPHONE,
};

pub const FORGEAUDIO_HDA_BACKEND_VERSION: u32 = 1;
const HDA_CLASS_MULTIMEDIA: u8 = 0x04;
const HDA_SUBCLASS_AUDIO: u8 = 0x03;
const HDA_MMIO_BYTES: u64 = 0x4000;
const HDA_WAIT_SPINS: usize = 20_000_000;
const HDA_DOMAIN_ID: u16 = 0x1544;
const HDA_DOMAIN_GENERATION: u32 = 1;

// Intel HD Audio controller register layout.
const REG_GCAP: u64 = 0x00;
const REG_VMIN: u64 = 0x02;
const REG_VMAJ: u64 = 0x03;
const REG_GCTL: u64 = 0x08;
const REG_STATESTS: u64 = 0x0e;
const REG_INTCTL: u64 = 0x20;
const REG_INTSTS: u64 = 0x24;
const REG_CORBLBASE: u64 = 0x40;
const REG_CORBUBASE: u64 = 0x44;
const REG_CORBWP: u64 = 0x48;
const REG_CORBRP: u64 = 0x4a;
const REG_CORBCTL: u64 = 0x4c;
const REG_CORBSTS: u64 = 0x4d;
const REG_CORBSIZE: u64 = 0x4e;
const REG_RIRBLBASE: u64 = 0x50;
const REG_RIRBUBASE: u64 = 0x54;
const REG_RIRBWP: u64 = 0x58;
const REG_RINTCNT: u64 = 0x5a;
const REG_RIRBCTL: u64 = 0x5c;
const REG_RIRBSTS: u64 = 0x5d;
const REG_RIRBSIZE: u64 = 0x5e;
const STREAM_BASE: u64 = 0x80;
const STREAM_STRIDE: u64 = 0x20;

const GCTL_CRST: u32 = 1 << 0;
const INTCTL_GIE: u32 = 1 << 31;
const INTSTS_CIS: u32 = 1 << 30;
const CORBCTL_RUN: u8 = 1 << 1;
const RIRBCTL_IRQ: u8 = 1 << 0;
const RIRBCTL_RUN: u8 = 1 << 1;
const CORBRP_RESET: u16 = 1 << 15;
const RIRBWP_RESET: u16 = 1 << 15;
const CORBSTS_MEMORY_ERROR: u8 = 1 << 0;
const RIRBSTS_RESPONSE_IRQ: u8 = 1 << 0;
const RIRBSTS_OVERRUN: u8 = 1 << 2;
const RIRBSTS_ACK_MASK: u8 = RIRBSTS_RESPONSE_IRQ | RIRBSTS_OVERRUN;
const STREAM_CTL_SRST: u8 = 1 << 0;
const STREAM_CTL_RUN: u8 = 1 << 1;
const STREAM_CTL_IOCE: u8 = 1 << 2;
const STREAM_STATUS_ACK: u8 = (1 << 2) | (1 << 3) | (1 << 4);
const HDA_FORMAT_48K_S16_STEREO: u16 = 0x0011;

// HDA codec verbs/parameters used by the bounded discovery/configuration path.
const VERB_GET_PARAMETER: u16 = 0x0f00;
const VERB_GET_CONNECTION_LIST: u16 = 0x0f02;
const VERB_SET_POWER_STATE: u16 = 0x0705;
const VERB_SET_STREAM_CHANNEL: u16 = 0x0706;
const VERB_SET_PIN_WIDGET_CONTROL: u16 = 0x0707;
const VERB_SET_CONVERTER_FORMAT_4BIT: u8 = 0x2;
const PARAM_VENDOR_ID: u8 = 0x00;
const PARAM_SUBORDINATE_NODE_COUNT: u8 = 0x04;
const PARAM_FUNCTION_GROUP_TYPE: u8 = 0x05;
const PARAM_AUDIO_WIDGET_CAPS: u8 = 0x09;
const PARAM_PIN_CAPS: u8 = 0x0c;
const PARAM_CONNECTION_LIST_LENGTH: u8 = 0x0e;
const WIDGET_AUDIO_OUTPUT: u8 = 0;
const WIDGET_AUDIO_INPUT: u8 = 1;
const WIDGET_PIN_COMPLEX: u8 = 4;
const PINCTL_OUTPUT_ENABLE: u8 = 1 << 6;
const PINCTL_INPUT_ENABLE: u8 = 1 << 5;

const IOVA_CORB: u64 = 0x0002_0000;
const IOVA_RIRB: u64 = 0x0002_1000;
const IOVA_PLAYBACK_BDL: u64 = 0x0002_2000;
const IOVA_CAPTURE_BDL: u64 = 0x0002_3000;
const IOVA_PLAYBACK_RING: u64 = 0x0003_0000;
const IOVA_CAPTURE_RING: u64 = 0x0004_0000;

const TEST_PERIOD_FRAMES: u32 = 1024;
const TEST_PERIOD_COUNT: usize = 4;
const TEST_PERIODS_PER_DIRECTION: u64 = 2;

#[derive(Clone, Copy, Debug)]
pub struct HdaQualificationReport {
    pub backend_version: u32,
    pub pci_vendor: u16,
    pub pci_device: u16,
    pub controller_reset: bool,
    pub corb_ready: bool,
    pub rirb_ready: bool,
    pub codec_count: u8,
    pub widget_count: u8,
    pub playback_converter: u8,
    pub capture_converter: u8,
    pub translated_dma: bool,
    pub bdl_ready: bool,
    pub msi_enabled: bool,
    pub hardware_interrupts: u64,
    pub stream_interrupts: u64,
    pub playback_periods: u64,
    pub capture_periods: u64,
    pub playback_frames: u64,
    pub capture_frames: u64,
    pub capture_memory_changed: bool,
    pub forgeaudio_device_registered: bool,
    pub forgeaudio_endpoints: u32,
    pub physical_silicon: bool,
}

#[derive(Clone, Copy)]
struct HdaIrqRuntime {
    active: bool,
    mmio: u64,
    device: DeviceId,
    playback_stream_base: u64,
    capture_stream_base: u64,
    playback_stream_bit: u8,
    capture_stream_bit: u8,
}
impl HdaIrqRuntime {
    const EMPTY: Self = Self {
        active: false,
        mmio: 0,
        device: DeviceId(0),
        playback_stream_base: 0,
        capture_stream_base: 0,
        playback_stream_bit: 0,
        capture_stream_bit: 0,
    };
}

static IRQ_RUNTIME: SpinLock<HdaIrqRuntime> = SpinLock::new(HdaIrqRuntime::EMPTY);
static IRQ_EVENTS: AtomicU64 = AtomicU64::new(0);
static STREAM_IRQ_EVENTS: AtomicU64 = AtomicU64::new(0);
static COMMAND_IRQ_EVENTS: AtomicU64 = AtomicU64::new(0);

fn hda_irq_handler(_vector: u8, device: DeviceId) -> Result<(), &'static str> {
    let runtime = *IRQ_RUNTIME.lock();
    if !runtime.active || runtime.device != device || runtime.mmio == 0 {
        return Err("HDA interrupt arrived without active controller ownership");
    }
    let status = read32(runtime.mmio + REG_INTSTS);
    let playback_mask = 1u32 << runtime.playback_stream_bit;
    let capture_mask = 1u32 << runtime.capture_stream_bit;
    let mut stream_event = false;
    if status & playback_mask != 0 {
        write8(runtime.playback_stream_base + 3, STREAM_STATUS_ACK);
        stream_event = true;
    }
    if status & capture_mask != 0 {
        write8(runtime.capture_stream_base + 3, STREAM_STATUS_ACK);
        stream_event = true;
    }
    if stream_event {
        STREAM_IRQ_EVENTS.fetch_add(1, Ordering::AcqRel);
    }
    if status & INTSTS_CIS != 0 {
        let rirb_status = read8(runtime.mmio + REG_RIRBSTS);
        if rirb_status != 0 { write8(runtime.mmio + REG_RIRBSTS, rirb_status); }
        let corb_status = read8(runtime.mmio + REG_CORBSTS);
        if corb_status != 0 { write8(runtime.mmio + REG_CORBSTS, corb_status); }
        COMMAND_IRQ_EVENTS.fetch_add(1, Ordering::AcqRel);
    }
    IRQ_EVENTS.fetch_add(1, Ordering::AcqRel);
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct CodecTopology {
    codec_address: u8,
    vendor_id: u32,
    function_group: u8,
    playback_converter: u8,
    capture_converter: u8,
    playback_pin: u8,
    capture_pin: u8,
    widget_count: u8,
}
impl CodecTopology {
    const EMPTY: Self = Self {
        codec_address: 0,
        vendor_id: 0,
        function_group: 0,
        playback_converter: 0,
        capture_converter: 0,
        playback_pin: 0,
        capture_pin: 0,
        widget_count: 0,
    };
}

struct CommandRings {
    corb_physical: u64,
    rirb_physical: u64,
    corb_entries: u16,
    rirb_entries: u16,
    corb_wp: u16,
    rirb_rp: u16,
}

impl CommandRings {
    fn initialize(mmio: u64, corb_physical: u64, rirb_physical: u64) -> Result<Self, &'static str> {
        write8(mmio + REG_CORBCTL, 0);
        write8(mmio + REG_RIRBCTL, 0);
        write8(mmio + REG_CORBSTS, CORBSTS_MEMORY_ERROR);
        write8(mmio + REG_RIRBSTS, RIRBSTS_ACK_MASK);

        // CORBSIZE/RIRBSIZE are allowed to be fixed/read-only.  Intel PCH and
        // QEMU ICH9 HDA expose 0x42: 256-entry capability with 256 selected.
        // Consume the controller-selected geometry rather than blindly writing
        // the size registers and generating guest-errors on fixed controllers.
        let corb_size = read8(mmio + REG_CORBSIZE);
        let rirb_size = read8(mmio + REG_RIRBSIZE);
        let corb_entries = selected_ring_entries(corb_size)?;
        let rirb_entries = selected_ring_entries(rirb_size)?;
        if corb_entries < 16 || rirb_entries < 16 {
            return Err("HDA command rings expose fewer than 16 selected entries");
        }
        if !ring_selection_is_advertised(corb_size, corb_entries)
            || !ring_selection_is_advertised(rirb_size, rirb_entries)
        {
            return Err("HDA selected command-ring size is not advertised by capability bits");
        }

        zero_bytes(corb_physical, FRAME_SIZE as usize);
        zero_bytes(rirb_physical, FRAME_SIZE as usize);
        memory_barrier();

        write32(mmio + REG_CORBLBASE, IOVA_CORB as u32);
        write32(mmio + REG_CORBUBASE, (IOVA_CORB >> 32) as u32);
        write32(mmio + REG_RIRBLBASE, IOVA_RIRB as u32);
        write32(mmio + REG_RIRBUBASE, (IOVA_RIRB >> 32) as u32);
        if read32(mmio + REG_CORBLBASE) & !0x7f != IOVA_CORB as u32
            || read32(mmio + REG_RIRBLBASE) & !0x7f != IOVA_RIRB as u32
        {
            return Err("HDA CORB/RIRB base-address readback failed");
        }

        // CORBRP reset is a two-phase handshake: assert reset, observe it,
        // deassert it, then observe the clear.  RIRBWP reset is write-only and
        // must always read back with a zero pointer after reset.
        write16(mmio + REG_CORBRP, CORBRP_RESET);
        wait16(mmio + REG_CORBRP, CORBRP_RESET, true, "HDA CORBRP reset assert timed out")?;
        write16(mmio + REG_CORBRP, 0);
        wait16(mmio + REG_CORBRP, CORBRP_RESET, false, "HDA CORBRP reset deassert timed out")?;
        if read16(mmio + REG_CORBRP) & 0x00ff != 0 {
            return Err("HDA CORBRP did not reset to zero");
        }
        write16(mmio + REG_CORBWP, 0);
        if read16(mmio + REG_CORBWP) & 0x00ff != 0 {
            return Err("HDA CORBWP did not initialize to zero");
        }
        write16(mmio + REG_RIRBWP, RIRBWP_RESET);
        if read16(mmio + REG_RIRBWP) & 0x00ff != 0 {
            return Err("HDA RIRBWP did not reset to zero");
        }
        write16(mmio + REG_RINTCNT, 1);
        if read16(mmio + REG_RINTCNT) & 0x00ff != 1 {
            return Err("HDA RINTCNT did not accept response threshold");
        }

        memory_barrier();
        write8(mmio + REG_RIRBCTL, RIRBCTL_RUN | RIRBCTL_IRQ);
        write8(mmio + REG_CORBCTL, CORBCTL_RUN);
        if read8(mmio + REG_CORBCTL) & CORBCTL_RUN == 0 || read8(mmio + REG_RIRBCTL) & RIRBCTL_RUN == 0 {
            return Err("HDA CORB/RIRB DMA engines did not start");
        }
        serial::println(format_args!(
            "[K15HDA] command rings: CORBSIZE={:#04x} RIRBSIZE={:#04x} corb_entries={} rirb_entries={} fixed_selection=true",
            corb_size, rirb_size, corb_entries, rirb_entries
        ));
        Ok(Self { corb_physical, rirb_physical, corb_entries, rirb_entries, corb_wp: 0, rirb_rp: 0 })
    }

    fn stop(&self, mmio: u64) {
        write8(mmio + REG_CORBCTL, 0);
        write8(mmio + REG_RIRBCTL, 0);
    }

    fn acknowledge_rirb_status(mmio: u64) {
        let status = read8(mmio + REG_RIRBSTS) & RIRBSTS_ACK_MASK;
        if status != 0 { write8(mmio + REG_RIRBSTS, status); }
    }

    fn command(&mut self, mmio: u64, command: u32, expected_codec: u8) -> Result<u32, &'static str> {
        let next = (self.corb_wp + 1) % self.corb_entries;
        unsafe { ptr::write_volatile((self.corb_physical as *mut u32).add(next as usize), command); }
        memory_barrier();
        write16(mmio + REG_CORBWP, next);
        self.corb_wp = next;
        let mut fetched = false;

        for _ in 0..HDA_WAIT_SPINS {
            let corb_status = read8(mmio + REG_CORBSTS);
            if corb_status & CORBSTS_MEMORY_ERROR != 0 {
                write8(mmio + REG_CORBSTS, CORBSTS_MEMORY_ERROR);
                serial::println(format_args!(
                    "[K15HDA] CORB DMA error: command={:#010x} CORBRP={:#06x} CORBWP={:#06x} CORBSTS={:#04x}",
                    command, read16(mmio + REG_CORBRP), read16(mmio + REG_CORBWP), corb_status
                ));
                return Err("HDA CORB DMA memory error");
            }
            if read16(mmio + REG_CORBRP) & 0x00ff == next { fetched = true; }

            let hardware_wp = (read16(mmio + REG_RIRBWP) & 0x00ff) % self.rirb_entries;
            let mut consumed = false;
            while self.rirb_rp != hardware_wp {
                self.rirb_rp = (self.rirb_rp + 1) % self.rirb_entries;
                let entry = unsafe { ptr::read_volatile((self.rirb_physical as *const u64).add(self.rirb_rp as usize)) };
                let response = entry as u32;
                let extended = (entry >> 32) as u32;
                let unsolicited = extended & (1 << 4) != 0;
                let codec = (extended & 0x0f) as u8;
                consumed = true;
                if !unsolicited && codec == expected_codec {
                    // We are polling the command path synchronously.  Ack the
                    // RIRB response status here as well as in the ISR so the
                    // controller may accept the next CORB verb even if MSI is
                    // not serviced until after this polling loop exits.
                    Self::acknowledge_rirb_status(mmio);
                    return Ok(response);
                }
            }
            if consumed { Self::acknowledge_rirb_status(mmio); }
            let rirb_status = read8(mmio + REG_RIRBSTS);
            if rirb_status & RIRBSTS_OVERRUN != 0 {
                Self::acknowledge_rirb_status(mmio);
                return Err("HDA RIRB overrun while waiting for codec response");
            }
            core::hint::spin_loop();
        }

        serial::println(format_args!(
            "[K15HDA] command timeout diagnostic: cmd={:#010x} codec={} fetched={} CORBRP={:#06x} CORBWP={:#06x} CORBCTL={:#04x} CORBSTS={:#04x} RIRBWP={:#06x} rirb_rp={} RIRBCTL={:#04x} RIRBSTS={:#04x} RINTCNT={} CORBSIZE={:#04x} RIRBSIZE={:#04x}",
            command, expected_codec, fetched, read16(mmio + REG_CORBRP), read16(mmio + REG_CORBWP),
            read8(mmio + REG_CORBCTL), read8(mmio + REG_CORBSTS), read16(mmio + REG_RIRBWP), self.rirb_rp,
            read8(mmio + REG_RIRBCTL), read8(mmio + REG_RIRBSTS), read16(mmio + REG_RINTCNT) & 0xff,
            read8(mmio + REG_CORBSIZE), read8(mmio + REG_RIRBSIZE)
        ));
        if fetched {
            Err("HDA CORB command fetched but timed out waiting for RIRB response")
        } else {
            Err("HDA CORB DMA fetch timed out before codec command execution")
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct BdlEntry {
    address: u64,
    length: u32,
    flags: u32,
}

pub fn initialize_and_qualify(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
) -> Result<HdaQualificationReport, &'static str> {
    let hda = pci::find_first(|f| f.class_code == HDA_CLASS_MULTIMEDIA && f.subclass == HDA_SUBCLASS_AUDIO)
        .ok_or("K15.4 requires a real PCI HDA controller on the qualification target")?;
    let bar0 = pci::memory_bar_base(hda, 0).ok_or("HDA controller BAR0 is unavailable")?;
    if bar0 == 0 { return Err("HDA controller BAR0 is zero"); }
    let mmio = paging::map_kernel_mmio(allocator, kernel_cr3, bar0, HDA_MMIO_BYTES)?;
    pci::disable_bus_master(hda);
    pci::enable_memory_decode(hda);

    let (forge_device, _forge_driver) = forgebus::claim_pci_function(hda, b"forgeaudio-hda", 2)?;
    forgebus::establish_dma_domain(forge_device, 64, true)?;

    let gcap = read16(mmio + REG_GCAP);
    let input_streams = ((gcap >> 8) & 0x0f) as u8;
    let output_streams = ((gcap >> 12) & 0x0f) as u8;
    if input_streams == 0 || output_streams == 0 {
        return Err("HDA controller lacks required playback/capture stream descriptors");
    }
    let version_major = read8(mmio + REG_VMAJ);
    let version_minor = read8(mmio + REG_VMIN);
    controller_reset(mmio)?;
    let codec_mask = read16(mmio + REG_STATESTS) & 0x7fff;
    if codec_mask == 0 { return Err("HDA controller reports no codec after reset"); }

    let playback_stream_bit = input_streams;
    let capture_stream_bit = 0u8;
    let playback_stream_base = mmio + STREAM_BASE + u64::from(playback_stream_bit) * STREAM_STRIDE;
    let capture_stream_base = mmio + STREAM_BASE;

    serial::println(format_args!(
        "[K15HDA] controller: pci={:04x}:{:04x} bdf={:02x}:{:02x}.{} version={}.{} gcap={:#06x} inputs={} outputs={} reset=true codec_mask={:#06x}",
        hda.vendor_id, hda.device_id, hda.bus, hda.device, hda.function,
        version_major, version_minor, gcap, input_streams, output_streams, codec_mask
    ));

    let corb_page = allocate_zeroed_page(allocator)?;
    let rirb_page = match allocate_zeroed_page(allocator) {
        Ok(v) => v,
        Err(e) => { let _ = allocator.deallocate_frame(corb_page); return Err(e); }
    };
    let playback_bdl = match allocate_zeroed_page(allocator) {
        Ok(v) => v,
        Err(e) => { let _ = allocator.deallocate_frame(corb_page); let _ = allocator.deallocate_frame(rirb_page); return Err(e); }
    };
    let capture_bdl = match allocate_zeroed_page(allocator) {
        Ok(v) => v,
        Err(e) => {
            let _ = allocator.deallocate_frame(corb_page); let _ = allocator.deallocate_frame(rirb_page);
            let _ = allocator.deallocate_frame(playback_bdl); return Err(e);
        }
    };

    let mut playback = match AudioDmaTransport::allocate(
        allocator, kernel_cr3, AudioDirection::Playback, 4, TEST_PERIOD_FRAMES, TEST_PERIOD_COUNT,
    ) {
        Ok(v) => Some(v),
        Err(e) => {
            release_aux_pages(allocator, &[corb_page, rirb_page, playback_bdl, capture_bdl]);
            return Err(e);
        }
    };
    let mut capture = match AudioDmaTransport::allocate(
        allocator, kernel_cr3, AudioDirection::Capture, 4, TEST_PERIOD_FRAMES, TEST_PERIOD_COUNT,
    ) {
        Ok(v) => Some(v),
        Err(e) => {
            if let Some(p) = playback.take() { let _ = p.release(allocator, kernel_cr3); }
            release_aux_pages(allocator, &[corb_page, rirb_page, playback_bdl, capture_bdl]);
            return Err(e);
        }
    };

    for index in 0..2usize {
        fill_playback_period(playback.as_ref().ok_or("playback transport disappeared")?, index, index as u32)?;
        playback.as_mut().ok_or("playback transport disappeared")?.queue_playback_period(index, TEST_PERIOD_FRAMES)?;
    }
    for index in 0..2usize {
        poison_capture_period(capture.as_ref().ok_or("capture transport disappeared")?, index)?;
    }

    let playback_geometry = playback.as_ref().ok_or("playback transport disappeared before HDA mapping")?;
    let capture_geometry = capture.as_ref().ok_or("capture transport disappeared before HDA mapping")?;
    let playback_ring_physical = playback_geometry.ring_physical_base();
    let playback_ring_bytes = playback_geometry.ring_mapped_bytes();
    let capture_ring_physical = capture_geometry.ring_physical_base();
    let capture_ring_bytes = capture_geometry.ring_mapped_bytes();
    let regions = [
        TemporaryDmaRegion { physical_base: corb_page, byte_length: FRAME_SIZE, iova_base: IOVA_CORB, device_read: true, device_write: false },
        TemporaryDmaRegion { physical_base: rirb_page, byte_length: FRAME_SIZE, iova_base: IOVA_RIRB, device_read: false, device_write: true },
        TemporaryDmaRegion { physical_base: playback_bdl, byte_length: FRAME_SIZE, iova_base: IOVA_PLAYBACK_BDL, device_read: true, device_write: false },
        TemporaryDmaRegion { physical_base: capture_bdl, byte_length: FRAME_SIZE, iova_base: IOVA_CAPTURE_BDL, device_read: true, device_write: false },
        TemporaryDmaRegion { physical_base: playback_ring_physical, byte_length: playback_ring_bytes, iova_base: IOVA_PLAYBACK_RING, device_read: true, device_write: false },
        TemporaryDmaRegion { physical_base: capture_ring_physical, byte_length: capture_ring_bytes, iova_base: IOVA_CAPTURE_RING, device_read: false, device_write: true },
    ];

    let mut topology = CodecTopology::EMPTY;
    let mut msi_enabled = false;
    let mut irq_vector = 0u8;
    let mut playback_periods = 0u64;
    let mut capture_periods = 0u64;
    let mut capture_changed = false;
    let mut command_rings_ready = false;
    let mut stream_bdl_ready = false;

    let hardware_result = translated_dma::with_temporary_translated_domain(
        allocator,
        kernel_cr3,
        hda,
        HDA_DOMAIN_ID,
        HDA_DOMAIN_GENERATION,
        &regions,
        |domain| {
            if !domain.hardware_translated || domain.requester == 0 || domain.mapped_pages < 10 {
                return Err("HDA translated DMA window did not arm exact requester mappings");
            }
            let pci_address = PciAddress::new(0, hda.bus, hda.device, hda.function)?;
            let bsp_apic = percpu::apic_id(percpu::bsp_index()).ok_or("BSP APIC id unavailable for HDA MSI")?;
            let playback_ref = playback.as_mut().ok_or("playback transport disappeared")?;
            let capture_ref = capture.as_mut().ok_or("capture transport disappeared")?;
            let playback_lease = DmaIsolationLease::new_translated(
                domain.requester, domain.domain_id, IOVA_PLAYBACK_RING,
                playback_ref.ring_mapped_bytes(), playback_ref.ring_physical_base(),
                playback_ref.ring_mapped_bytes(), AudioDmaAccess::DeviceRead, domain.generation,
            )?;
            let capture_lease = DmaIsolationLease::new_translated(
                domain.requester, domain.domain_id, IOVA_CAPTURE_RING,
                capture_ref.ring_mapped_bytes(), capture_ref.ring_physical_base(),
                capture_ref.ring_mapped_bytes(), AudioDmaAccess::DeviceWrite, domain.generation,
            )?;
            playback_ref.arm_hardware(&playback_lease)?;
            if let Err(error) = capture_ref.arm_hardware(&capture_lease) {
                let _ = playback_ref.disarm_hardware();
                return Err(error);
            }

            let route = match kernel_runtime::with_runtime(|runtime| {
                let route = runtime.interrupts.allocate(forge_device, percpu::bsp_index() as u16, false)?;
                if let Err(error) = runtime.interrupts.register_handler(route.vector, forge_device, hda_irq_handler) {
                    let _ = runtime.interrupts.release(route.vector, forge_device);
                    return Err(error);
                }
                if let Err(error) = runtime.interrupts.enable(route.vector, forge_device) {
                    let _ = runtime.interrupts.release(route.vector, forge_device);
                    return Err(error);
                }
                Ok::<_, &'static str>(route)
            }) {
                Ok(value) => value,
                Err(error) => {
                    let _ = capture_ref.disarm_hardware();
                    let _ = playback_ref.disarm_hardware();
                    return Err(error);
                }
            };
            irq_vector = route.vector;
            let mut msi_lease = match msi::enable_msi(pci_address, bsp_apic, route.vector) {
                Ok(value) => value,
                Err(error) => {
                    let _ = kernel_runtime::with_runtime(|runtime| {
                        let _ = runtime.interrupts.mask(route.vector, forge_device);
                        runtime.interrupts.release(route.vector, forge_device)
                    });
                    let _ = capture_ref.disarm_hardware();
                    let _ = playback_ref.disarm_hardware();
                    return Err(error);
                }
            };
            msi_enabled = msi_lease.enabled;
            *IRQ_RUNTIME.lock() = HdaIrqRuntime {
                active: true,
                mmio,
                device: forge_device,
                playback_stream_base,
                capture_stream_base,
                playback_stream_bit,
                capture_stream_bit,
            };
            IRQ_EVENTS.store(0, Ordering::Release);
            STREAM_IRQ_EVENTS.store(0, Ordering::Release);
            COMMAND_IRQ_EVENTS.store(0, Ordering::Release);
            write32(mmio + REG_INTCTL, INTCTL_GIE | (1u32 << playback_stream_bit) | (1u32 << capture_stream_bit));

            let mut rings: Option<CommandRings> = None;
            let operation_result = (|| -> Result<(), &'static str> {
                rings = Some(CommandRings::initialize(mmio, corb_page, rirb_page)?);
                command_rings_ready = true;
                let rings_ref = rings.as_mut().ok_or("HDA command rings disappeared after initialization")?;
                topology = discover_topology(mmio, rings_ref, codec_mask)?;
                configure_codec(mmio, rings_ref, topology)?;
                serial::println(format_args!(
                    "[K15HDA] command+codec: CORB=true RIRB=true command_irqs={} codec={} vendor={:#010x} fg={} widgets={} playback_converter={} capture_converter={} playback_pin={} capture_pin={}",
                    COMMAND_IRQ_EVENTS.load(Ordering::Acquire), topology.codec_address, topology.vendor_id,
                    topology.function_group, topology.widget_count, topology.playback_converter,
                    topology.capture_converter, topology.playback_pin, topology.capture_pin
                ));

                reset_stream(playback_stream_base)?;
                reset_stream(capture_stream_base)?;
                program_stream_static(playback_stream_base, IOVA_PLAYBACK_BDL, playback_ref.period_bytes(), 1)?;
                program_stream_static(capture_stream_base, IOVA_CAPTURE_BDL, capture_ref.period_bytes(), 2)?;
                stream_bdl_ready = true;

                for _ in 0..TEST_PERIODS_PER_DIRECTION {
                    let period = playback_ref.backend_acquire_next()?;
                    write_bdl_single(playback_bdl, period.device_address, period.byte_length)?;
                    let before = STREAM_IRQ_EVENTS.load(Ordering::Acquire);
                    run_stream_one_period(mmio, playback_stream_base, playback_stream_bit, before)?;
                    // Completion is legal only after the HDA stream interrupt advanced.
                    if STREAM_IRQ_EVENTS.load(Ordering::Acquire) <= before {
                        return Err("HDA playback period completed without hardware stream interrupt evidence");
                    }
                    playback_ref.backend_complete_period(period.index, period.frames)?;
                    playback_periods = playback_periods.saturating_add(1);
                }

                for index in 0..TEST_PERIODS_PER_DIRECTION as usize {
                    let period = capture_ref.backend_acquire_next()?;
                    write_bdl_single(capture_bdl, period.device_address, period.byte_length)?;
                    let before = STREAM_IRQ_EVENTS.load(Ordering::Acquire);
                    run_stream_one_period(mmio, capture_stream_base, capture_stream_bit, before)?;
                    if STREAM_IRQ_EVENTS.load(Ordering::Acquire) <= before {
                        return Err("HDA capture period completed without hardware stream interrupt evidence");
                    }
                    capture_ref.backend_complete_period(period.index, period.frames)?;
                    if capture_period_changed(capture_ref, index)? { capture_changed = true; }
                    capture_ref.release_capture_period(period.index)?;
                    capture_periods = capture_periods.saturating_add(1);
                }
                Ok(())
            })();

            // Cleanup is unconditional. A failed hardware operation must never
            // strand a device-owned K15.3 period, MSI route, command ring, or
            // hardware-armed transport while the VT-d window is revoked.
            let _ = stop_stream(playback_stream_base);
            let _ = stop_stream(capture_stream_base);
            if let Some(rings_ref) = rings.as_mut() {
                if topology.playback_converter != 0 {
                    let _ = set_converter_stream(mmio, rings_ref, topology.codec_address, topology.playback_converter, 0);
                }
                if topology.capture_converter != 0 {
                    let _ = set_converter_stream(mmio, rings_ref, topology.codec_address, topology.capture_converter, 0);
                }
                rings_ref.stop(mmio);
            } else {
                write8(mmio + REG_CORBCTL, 0);
                write8(mmio + REG_RIRBCTL, 0);
            }
            write32(mmio + REG_INTCTL, 0);
            let playback_abort = playback_ref.backend_abort_inflight();
            let capture_abort = capture_ref.backend_abort_inflight();
            let playback_disarm = playback_ref.disarm_hardware();
            let capture_disarm = capture_ref.disarm_hardware();
            msi_lease.disable();
            *IRQ_RUNTIME.lock() = HdaIrqRuntime::EMPTY;
            let route_release = kernel_runtime::with_runtime(|runtime| {
                let _ = runtime.interrupts.mask(route.vector, forge_device);
                runtime.interrupts.release(route.vector, forge_device)
            });

            if let Err(error) = operation_result { return Err(error); }
            if let Err(error) = playback_abort { return Err(error); }
            if let Err(error) = capture_abort { return Err(error); }
            if let Err(error) = playback_disarm { return Err(error); }
            if let Err(error) = capture_disarm { return Err(error); }
            route_release?;
            Ok(())
        },
    );

    // Always leave the HDA requester fenced, even if qualification failed.
    pci::disable_bus_master(hda);
    write32(mmio + REG_INTCTL, 0);
    *IRQ_RUNTIME.lock() = HdaIrqRuntime::EMPTY;

    let playback_snapshot = playback.as_ref().map(|p| p.snapshot());
    let capture_snapshot = capture.as_ref().map(|p| p.snapshot());
    let release_playback = playback.take().map(|p| p.release(allocator, kernel_cr3));
    let release_capture = capture.take().map(|p| p.release(allocator, kernel_cr3));
    release_aux_pages(allocator, &[corb_page, rirb_page, playback_bdl, capture_bdl]);

    hardware_result?;
    if let Some(Err(e)) = release_playback { return Err(e); }
    if let Some(Err(e)) = release_capture { return Err(e); }
    if !command_rings_ready || !stream_bdl_ready || !msi_enabled {
        return Err("HDA backend did not complete command/BDL/MSI activation");
    }
    if IRQ_EVENTS.load(Ordering::Acquire) == 0 || STREAM_IRQ_EVENTS.load(Ordering::Acquire) < 4 {
        return Err("HDA hardware MSI/stream completion evidence is incomplete");
    }
    if playback_periods != TEST_PERIODS_PER_DIRECTION || capture_periods != TEST_PERIODS_PER_DIRECTION {
        return Err("HDA playback/capture period completion count is incomplete");
    }
    if !capture_changed { return Err("HDA capture DMA did not modify translated capture memory"); }
    let playback_frames = playback_snapshot.ok_or("playback DMA snapshot missing")?.frame_position;
    let capture_frames = capture_snapshot.ok_or("capture DMA snapshot missing")?.frame_position;
    if playback_frames != u64::from(TEST_PERIOD_FRAMES) * TEST_PERIODS_PER_DIRECTION
        || capture_frames != u64::from(TEST_PERIOD_FRAMES) * TEST_PERIODS_PER_DIRECTION
    {
        return Err("HDA DMA frame-position accounting mismatch");
    }

    let registered = register_forgeaudio_device(hda)?;
    forgebus::mark_device_online(forge_device)?;
    let device = forgeaudio::enumerate_device(registered).ok_or("registered HDA device disappeared from ForgeAudio")?;
    if device.endpoint_count != 2 { return Err("HDA ForgeAudio registry did not expose playback+capture endpoints"); }

    let report = HdaQualificationReport {
        backend_version: FORGEAUDIO_HDA_BACKEND_VERSION,
        pci_vendor: hda.vendor_id,
        pci_device: hda.device_id,
        controller_reset: true,
        corb_ready: true,
        rirb_ready: true,
        codec_count: codec_mask.count_ones() as u8,
        widget_count: topology.widget_count,
        playback_converter: topology.playback_converter,
        capture_converter: topology.capture_converter,
        translated_dma: true,
        bdl_ready: true,
        msi_enabled,
        hardware_interrupts: IRQ_EVENTS.load(Ordering::Acquire),
        stream_interrupts: STREAM_IRQ_EVENTS.load(Ordering::Acquire),
        playback_periods,
        capture_periods,
        playback_frames,
        capture_frames,
        capture_memory_changed: capture_changed,
        forgeaudio_device_registered: true,
        forgeaudio_endpoints: device.endpoint_count,
        physical_silicon: false,
    };

    serial::println(format_args!(
        "[K15HDA] DMA+IRQ: translated=true BDL=true MSI=true vector={:#04x} hw_irqs={} stream_irqs={} playback_periods={} capture_periods={} playback_frames={} capture_frames={} capture_memory_changed={} bus_master_after=false",
        irq_vector, report.hardware_interrupts, report.stream_interrupts, report.playback_periods,
        report.capture_periods, report.playback_frames, report.capture_frames, report.capture_memory_changed
    ));
    serial::println(format_args!(
        "[K15HDA] ForgeAudio registry: device=true endpoints={} playback=true capture=true backend=HDA placeholder=false",
        report.forgeaudio_endpoints
    ));
    serial::println(format_args!(
        "[K15OK] K15.4 ForgeAudio real HDA hardware backend qualified: pci=true reset=true CORB=true RIRB=true codecs=true widgets=true BDL=true translated_dma=true MSI=true irq=true playback=true capture=true registry=true fake_hw=false physical_silicon=false"
    ));
    Ok(report)
}

fn controller_reset(mmio: u64) -> Result<(), &'static str> {
    write8(mmio + REG_CORBCTL, 0);
    write8(mmio + REG_RIRBCTL, 0);
    let iss = ((read16(mmio + REG_GCAP) >> 8) & 0x0f) as u8;
    let oss = ((read16(mmio + REG_GCAP) >> 12) & 0x0f) as u8;
    for stream in 0..(iss + oss) {
        let base = STREAM_BASE + u64::from(stream) * STREAM_STRIDE;
        write8(base + mmio, read8(base + mmio) & !STREAM_CTL_RUN);
    }
    write32(mmio + REG_GCTL, read32(mmio + REG_GCTL) & !GCTL_CRST);
    wait32(mmio + REG_GCTL, GCTL_CRST, false, "HDA controller failed to enter reset")?;
    for _ in 0..10_000 { core::hint::spin_loop(); }
    write32(mmio + REG_GCTL, read32(mmio + REG_GCTL) | GCTL_CRST);
    wait32(mmio + REG_GCTL, GCTL_CRST, true, "HDA controller failed to leave reset")?;
    Ok(())
}

fn discover_topology(mmio: u64, rings: &mut CommandRings, codec_mask: u16) -> Result<CodecTopology, &'static str> {
    for codec in 0u8..15 {
        if codec_mask & (1u16 << codec) == 0 { continue; }
        let vendor = get_parameter(mmio, rings, codec, 0, PARAM_VENDOR_ID)?;
        let root_nodes = get_parameter(mmio, rings, codec, 0, PARAM_SUBORDINATE_NODE_COUNT)?;
        let first_fg = ((root_nodes >> 16) & 0xff) as u8;
        let fg_count = (root_nodes & 0xff) as u8;
        for fg_offset in 0..fg_count {
            let fg = first_fg.wrapping_add(fg_offset);
            let fg_type = get_parameter(mmio, rings, codec, fg, PARAM_FUNCTION_GROUP_TYPE)? as u8 & 0x7f;
            if fg_type != 1 { continue; }
            let subnodes = get_parameter(mmio, rings, codec, fg, PARAM_SUBORDINATE_NODE_COUNT)?;
            let first = ((subnodes >> 16) & 0xff) as u8;
            let count = (subnodes & 0xff) as u8;
            if count == 0 || count > 64 { return Err("HDA audio function group widget count is invalid"); }
            let mut topology = CodecTopology { codec_address: codec, vendor_id: vendor, function_group: fg, widget_count: count, ..CodecTopology::EMPTY };
            let mut pins = [0u8; 16];
            let mut pin_count = 0usize;
            for offset in 0..count {
                let nid = first.wrapping_add(offset);
                let caps = get_parameter(mmio, rings, codec, nid, PARAM_AUDIO_WIDGET_CAPS)?;
                let widget_type = ((caps >> 20) & 0x0f) as u8;
                match widget_type {
                    WIDGET_AUDIO_OUTPUT if topology.playback_converter == 0 => topology.playback_converter = nid,
                    WIDGET_AUDIO_INPUT if topology.capture_converter == 0 => topology.capture_converter = nid,
                    WIDGET_PIN_COMPLEX if pin_count < pins.len() => { pins[pin_count] = nid; pin_count += 1; },
                    _ => {}
                }
            }
            if topology.playback_converter == 0 || topology.capture_converter == 0 {
                return Err("HDA codec lacks required playback/capture converters");
            }
            for pin in &pins[..pin_count] {
                let _pin_caps = get_parameter(mmio, rings, codec, *pin, PARAM_PIN_CAPS)?;
                if topology.playback_pin == 0 && connection_contains(mmio, rings, codec, *pin, topology.playback_converter)? {
                    topology.playback_pin = *pin;
                }
            }
            if topology.capture_pin == 0 {
                for pin in &pins[..pin_count] {
                    if connection_contains(mmio, rings, codec, topology.capture_converter, *pin)? {
                        topology.capture_pin = *pin;
                        break;
                    }
                }
            }
            // QEMU and some minimal codecs may omit a connection list on a pin;
            // the converter DMA proof remains valid, but pin enable is applied
            // only when a path was actually discovered.
            return Ok(topology);
        }
    }
    Err("HDA audio function group was not discovered")
}

fn configure_codec(mmio: u64, rings: &mut CommandRings, topology: CodecTopology) -> Result<(), &'static str> {
    set_verb12(mmio, rings, topology.codec_address, topology.function_group, VERB_SET_POWER_STATE, 0)?;
    set_verb12(mmio, rings, topology.codec_address, topology.playback_converter, VERB_SET_POWER_STATE, 0)?;
    set_verb12(mmio, rings, topology.codec_address, topology.capture_converter, VERB_SET_POWER_STATE, 0)?;
    set_converter_format(mmio, rings, topology.codec_address, topology.playback_converter, HDA_FORMAT_48K_S16_STEREO)?;
    set_converter_format(mmio, rings, topology.codec_address, topology.capture_converter, HDA_FORMAT_48K_S16_STEREO)?;
    set_converter_stream(mmio, rings, topology.codec_address, topology.playback_converter, 1)?;
    set_converter_stream(mmio, rings, topology.codec_address, topology.capture_converter, 2)?;
    if topology.playback_pin != 0 {
        set_verb12(mmio, rings, topology.codec_address, topology.playback_pin, VERB_SET_PIN_WIDGET_CONTROL, PINCTL_OUTPUT_ENABLE)?;
    }
    if topology.capture_pin != 0 {
        set_verb12(mmio, rings, topology.codec_address, topology.capture_pin, VERB_SET_PIN_WIDGET_CONTROL, PINCTL_INPUT_ENABLE)?;
    }
    Ok(())
}

fn connection_contains(mmio: u64, rings: &mut CommandRings, codec: u8, nid: u8, target: u8) -> Result<bool, &'static str> {
    let length = get_parameter(mmio, rings, codec, nid, PARAM_CONNECTION_LIST_LENGTH)? as u8;
    let long_form = length & 0x80 != 0;
    let count = (length & 0x7f) as usize;
    if count == 0 || count > 32 { return Ok(false); }
    let per_response = if long_form { 2 } else { 4 };
    let mut index = 0usize;
    while index < count {
        let response = rings.command(mmio, verb12(codec, nid, VERB_GET_CONNECTION_LIST, index as u8), codec)?;
        for lane in 0..per_response {
            if index + lane >= count { break; }
            let value = if long_form {
                ((response >> (lane * 16)) & 0x7fff) as u8
            } else {
                ((response >> (lane * 8)) & 0x7f) as u8
            };
            if value == target { return Ok(true); }
        }
        index += per_response;
    }
    Ok(false)
}

fn get_parameter(mmio: u64, rings: &mut CommandRings, codec: u8, nid: u8, parameter: u8) -> Result<u32, &'static str> {
    rings.command(mmio, verb12(codec, nid, VERB_GET_PARAMETER, parameter), codec)
}
fn set_verb12(mmio: u64, rings: &mut CommandRings, codec: u8, nid: u8, verb: u16, payload: u8) -> Result<(), &'static str> {
    let _ = rings.command(mmio, verb12(codec, nid, verb, payload), codec)?;
    Ok(())
}
fn set_converter_format(mmio: u64, rings: &mut CommandRings, codec: u8, nid: u8, format: u16) -> Result<(), &'static str> {
    let _ = rings.command(mmio, verb4(codec, nid, VERB_SET_CONVERTER_FORMAT_4BIT, format), codec)?;
    Ok(())
}
fn set_converter_stream(mmio: u64, rings: &mut CommandRings, codec: u8, nid: u8, stream_tag: u8) -> Result<(), &'static str> {
    set_verb12(mmio, rings, codec, nid, VERB_SET_STREAM_CHANNEL, (stream_tag & 0x0f) << 4)
}
fn verb12(codec: u8, nid: u8, verb: u16, payload: u8) -> u32 {
    (u32::from(codec & 0x0f) << 28) | (u32::from(nid) << 20) | (u32::from(verb & 0x0fff) << 8) | u32::from(payload)
}
fn verb4(codec: u8, nid: u8, verb: u8, payload: u16) -> u32 {
    (u32::from(codec & 0x0f) << 28) | (u32::from(nid) << 20) | (u32::from(verb & 0x0f) << 16) | u32::from(payload)
}

fn reset_stream(base: u64) -> Result<(), &'static str> {
    write8(base, read8(base) & !STREAM_CTL_RUN);
    wait8(base, STREAM_CTL_RUN, false, "HDA stream failed to stop before reset")?;
    write8(base, (read8(base) & !STREAM_CTL_RUN) | STREAM_CTL_SRST);
    wait8(base, STREAM_CTL_SRST, true, "HDA stream reset assert timed out")?;
    write8(base, read8(base) & !STREAM_CTL_SRST);
    wait8(base, STREAM_CTL_SRST, false, "HDA stream reset deassert timed out")?;
    write8(base + 3, STREAM_STATUS_ACK);
    Ok(())
}

fn program_stream_static(base: u64, bdl_iova: u64, period_bytes: u32, stream_tag: u8) -> Result<(), &'static str> {
    if stream_tag == 0 || stream_tag > 15 || period_bytes == 0 { return Err("invalid HDA stream programming geometry"); }
    write8(base, STREAM_CTL_IOCE);
    write8(base + 2, stream_tag << 4);
    write32(base + 8, period_bytes);
    write16(base + 12, 0); // one BDL entry per owned ForgeAudio period
    write16(base + 18, HDA_FORMAT_48K_S16_STEREO);
    write32(base + 24, bdl_iova as u32);
    write32(base + 28, (bdl_iova >> 32) as u32);
    Ok(())
}

fn write_bdl_single(bdl_physical: u64, device_address: u64, byte_length: u32) -> Result<(), &'static str> {
    if device_address == 0 || byte_length == 0 { return Err("HDA BDL entry is empty"); }
    let entry = BdlEntry { address: device_address, length: byte_length, flags: 1 };
    unsafe { ptr::write_volatile(bdl_physical as *mut BdlEntry, entry); }
    memory_barrier();
    Ok(())
}

fn run_stream_one_period(mmio: u64, base: u64, stream_bit: u8, irq_before: u64) -> Result<(), &'static str> {
    if stream_bit >= 8 { return Err("HDA stream interrupt bit is outside controller range"); }
    let required_intctl = INTCTL_GIE | (1u32 << stream_bit);
    if read32(mmio + REG_INTCTL) & required_intctl != required_intctl {
        return Err("HDA stream/global interrupt enable was lost before DMA start");
    }

    write8(base + 3, STREAM_STATUS_ACK);
    write8(base, read8(base) | STREAM_CTL_IOCE | STREAM_CTL_RUN);
    wait8(base, STREAM_CTL_RUN, true, "HDA stream RUN did not assert")?;

    // K15.1 deliberately returns boot to IF=0 after its RT qualification.
    // Hardware MSI delivery cannot execute a Titanweave IDT handler while the
    // local CPU interrupt flag remains clear.  Open only a bounded local
    // interrupt window for the real HDA completion wait, then restore the
    // exact pre-wait IF state before continuing initialization.
    let flags_before = x86_64::read_rflags();
    let interrupts_were_enabled = x86_64::interrupts_enabled();
    if !interrupts_were_enabled { x86_64::enable_interrupts(); }

    let mut interrupt_seen = false;
    for _ in 0..HDA_WAIT_SPINS {
        if STREAM_IRQ_EVENTS.load(Ordering::Acquire) > irq_before {
            interrupt_seen = true;
            break;
        }
        core::hint::spin_loop();
    }

    if !interrupts_were_enabled { x86_64::disable_interrupts(); }

    if interrupt_seen {
        stop_stream(base)?;
        return Ok(());
    }

    let status = read8(base + 3);
    let position = read32(base + 4);
    let intctl = read32(mmio + REG_INTCTL);
    let intsts = read32(mmio + REG_INTSTS);
    let flags_after = x86_64::read_rflags();
    let _ = stop_stream(base);
    serial::println(format_args!(
        "[K15HDA] stream timeout diagnostic: status={:#04x} lpib={} stream_bit={} INTCTL={:#010x} INTSTS={:#010x} irq_before={} irq_now={} IF_before={} IF_after={}",
        status, position, stream_bit, intctl, intsts, irq_before, STREAM_IRQ_EVENTS.load(Ordering::Acquire),
        flags_before & (1 << 9) != 0, flags_after & (1 << 9) != 0
    ));
    Err("HDA stream did not produce an MSI completion")
}

fn stop_stream(base: u64) -> Result<(), &'static str> {
    write8(base, read8(base) & !STREAM_CTL_RUN);
    wait8(base, STREAM_CTL_RUN, false, "HDA stream RUN did not clear")
}

fn fill_playback_period(transport: &AudioDmaTransport, index: usize, phase: u32) -> Result<(), &'static str> {
    let period = transport.period_descriptor(index).ok_or("playback period is outside transport")?;
    if period.byte_length < 4 { return Err("playback period is too small for stereo PCM"); }
    let frames = period.byte_length as usize / 4;
    for frame in 0..frames {
        let high = ((frame as u32 + phase * 31) / 48) & 1 == 0;
        let sample: i16 = if high { 1200 } else { -1200 };
        unsafe {
            let dst = (period.virtual_address as *mut i16).add(frame * 2);
            ptr::write_volatile(dst, sample);
            ptr::write_volatile(dst.add(1), sample);
        }
    }
    memory_barrier();
    Ok(())
}

fn poison_capture_period(transport: &AudioDmaTransport, index: usize) -> Result<(), &'static str> {
    let period = transport.period_descriptor(index).ok_or("capture period is outside transport")?;
    unsafe { ptr::write_bytes(period.virtual_address as *mut u8, 0xa5, period.byte_length as usize); }
    memory_barrier();
    Ok(())
}

fn capture_period_changed(transport: &AudioDmaTransport, index: usize) -> Result<bool, &'static str> {
    let period = transport.period_descriptor(index).ok_or("capture period is outside transport")?;
    for offset in 0..period.byte_length as usize {
        if unsafe { ptr::read_volatile((period.virtual_address as *const u8).add(offset)) } != 0xa5 { return Ok(true); }
    }
    Ok(false)
}

fn register_forgeaudio_device(hda: PciFunction) -> Result<usize, &'static str> {
    let subsystem = pci::read_u32(hda.bus, hda.device, hda.function, 0x2c);
    let device = forgeaudio::register_device(AudioDeviceInfo {
        object_id: 0,
        generation: 0,
        flags: AUDIO_DEVICE_FLAG_PLAYBACK | AUDIO_DEVICE_FLAG_CAPTURE | AUDIO_DEVICE_FLAG_FULL_DUPLEX | AUDIO_DEVICE_FLAG_CLOCK_MASTER,
        vendor_id: hda.vendor_id,
        device_id: hda.device_id,
        subsystem_vendor_id: subsystem as u16,
        subsystem_device_id: (subsystem >> 16) as u16,
        bus: hda.bus,
        device: hda.device,
        function: hda.function,
        backend_kind: AUDIO_BACKEND_HDA,
        endpoint_count: 0,
        clock_domain: 1,
        name: fixed_name(b"ForgeAudio HDA Controller"),
        reserved2: 0,
    })?;
    let format_mask = AudioSampleFormat::S16.mask();
    let _ = forgeaudio::register_endpoint(device.object_id, AudioEndpointInfo {
        object_id: 0, device_object_id: 0, generation: 0,
        direction: AudioDirection::Playback as u32,
        flags: AUDIO_ENDPOINT_FLAG_DEFAULT | AUDIO_ENDPOINT_FLAG_LINE_LEVEL,
        min_channels: 2, max_channels: 2, min_rate_hz: 48_000, max_rate_hz: 48_000,
        format_mask, reserved: 0, name: fixed_name(b"HDA Playback"),
    })?;
    let _ = forgeaudio::register_endpoint(device.object_id, AudioEndpointInfo {
        object_id: 0, device_object_id: 0, generation: 0,
        direction: AudioDirection::Capture as u32,
        flags: AUDIO_ENDPOINT_FLAG_DEFAULT | AUDIO_ENDPOINT_FLAG_MICROPHONE,
        min_channels: 2, max_channels: 2, min_rate_hz: 48_000, max_rate_hz: 48_000,
        format_mask, reserved: 0, name: fixed_name(b"HDA Capture"),
    })?;
    for index in 0..forgeaudio::device_count() {
        if let Some(candidate) = forgeaudio::enumerate_device(index) {
            if candidate.object_id == device.object_id { return Ok(index); }
        }
    }
    Err("registered HDA device could not be resolved by ForgeAudio object id")
}

fn fixed_name(source: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let count = core::cmp::min(source.len(), out.len() - 1);
    out[..count].copy_from_slice(&source[..count]);
    out
}

fn selected_ring_entries(value: u8) -> Result<u16, &'static str> {
    match value & 0x03 {
        0 => Ok(2),
        1 => Ok(16),
        2 => Ok(256),
        _ => Err("HDA controller selected a reserved CORB/RIRB ring size"),
    }
}

fn ring_selection_is_advertised(value: u8, entries: u16) -> bool {
    match entries {
        2 => value & (1 << 4) != 0,
        16 => value & (1 << 5) != 0,
        256 => value & (1 << 6) != 0,
        _ => false,
    }
}

fn allocate_zeroed_page(allocator: &mut FrameAllocator<'_>) -> Result<u64, &'static str> {
    let page = allocator.allocate_frame().ok_or("HDA DMA metadata page allocation failed")?;
    zero_bytes(page, FRAME_SIZE as usize);
    Ok(page)
}
fn release_aux_pages(allocator: &mut FrameAllocator<'_>, pages: &[u64]) {
    for page in pages { if *page != 0 { let _ = allocator.deallocate_frame(*page); } }
}
fn zero_bytes(address: u64, bytes: usize) { unsafe { ptr::write_bytes(address as *mut u8, 0, bytes); } }
fn memory_barrier() { unsafe { asm!("mfence", options(nostack, preserves_flags)); } }

fn wait8(address: u64, mask: u8, set: bool, error: &'static str) -> Result<(), &'static str> {
    for _ in 0..HDA_WAIT_SPINS {
        if (read8(address) & mask != 0) == set { return Ok(()); }
        core::hint::spin_loop();
    }
    Err(error)
}
fn wait16(address: u64, mask: u16, set: bool, error: &'static str) -> Result<(), &'static str> {
    for _ in 0..HDA_WAIT_SPINS {
        if (read16(address) & mask != 0) == set { return Ok(()); }
        core::hint::spin_loop();
    }
    Err(error)
}
fn wait32(address: u64, mask: u32, set: bool, error: &'static str) -> Result<(), &'static str> {
    for _ in 0..HDA_WAIT_SPINS {
        if (read32(address) & mask != 0) == set { return Ok(()); }
        core::hint::spin_loop();
    }
    Err(error)
}
fn read8(address: u64) -> u8 { unsafe { ptr::read_volatile(address as *const u8) } }
fn read16(address: u64) -> u16 { unsafe { ptr::read_volatile(address as *const u16) } }
fn read32(address: u64) -> u32 { unsafe { ptr::read_volatile(address as *const u32) } }
fn write8(address: u64, value: u8) { unsafe { ptr::write_volatile(address as *mut u8, value); } }
fn write16(address: u64, value: u16) { unsafe { ptr::write_volatile(address as *mut u16, value); } }
fn write32(address: u64, value: u32) { unsafe { ptr::write_volatile(address as *mut u32, value); } }
