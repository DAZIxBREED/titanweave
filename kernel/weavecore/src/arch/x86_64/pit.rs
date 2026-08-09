use crate::arch::x86_64::pause;
use crate::arch::x86_64::port::{inb, outb};

const PIT_COMMAND: u16 = 0x43;
const PIT_CHANNEL_2: u16 = 0x42;
const PC_SPEAKER: u16 = 0x61;
const PIT_INPUT_HZ: u32 = 1_193_182;
const CALIBRATION_MILLISECONDS: u32 = 10;
const MAX_POLL_ITERATIONS: usize = 50_000_000;

/// Start PIT channel 2 in one-shot mode for a nominal 10 ms interval.
///
/// Channel 2 is used because it can be polled through port 0x61 without
/// enabling legacy PIC interrupts. K3 uses it only as a calibration reference;
/// the Local APIC timer owns normal scheduler ticks afterward.
pub fn start_calibration_window() {
    let count = ((PIT_INPUT_HZ as u64 * CALIBRATION_MILLISECONDS as u64) / 1000)
        .clamp(1, u16::MAX as u64) as u16;

    unsafe {
        let mut speaker = inb(PC_SPEAKER);
        speaker &= !0b11; // Gate low; speaker data disabled.
        outb(PC_SPEAKER, speaker);

        // Channel 2, low/high byte, mode 0 one-shot, binary counting.
        outb(PIT_COMMAND, 0b1011_0000);
        outb(PIT_CHANNEL_2, count as u8);
        outb(PIT_CHANNEL_2, (count >> 8) as u8);

        // Raise the gate to begin the countdown while keeping the speaker off.
        outb(PC_SPEAKER, speaker | 0b01);
    }
}

/// Wait until PIT channel 2 reaches terminal count.
pub fn wait_for_calibration_window() -> Result<(), &'static str> {
    // Mode 0 drives OUT low while counting and high at terminal count. First
    // observe the low phase so a stale high bit cannot fake a zero-length
    // calibration window.
    let mut observed_low = false;
    for _ in 0..MAX_POLL_ITERATIONS {
        let status = unsafe { inb(PC_SPEAKER) };
        if status & (1 << 5) == 0 {
            observed_low = true;
            break;
        }
        pause();
    }
    if !observed_low {
        return Err("PIT channel 2 never entered its counting phase");
    }

    for _ in 0..MAX_POLL_ITERATIONS {
        let status = unsafe { inb(PC_SPEAKER) };
        if status & (1 << 5) != 0 {
            return Ok(());
        }
        pause();
    }
    Err("PIT channel 2 calibration window timed out")
}

#[must_use]
pub const fn calibration_milliseconds() -> u32 {
    CALIBRATION_MILLISECONDS
}
