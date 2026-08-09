use crate::arch::x86_64::pit;
use crate::arch::x86_64::port::outb;
use crate::arch::x86_64::{pause, read_msr, write_msr};
use core::ptr;
use core::sync::atomic::{AtomicU64, Ordering};

const IA32_APIC_BASE_MSR: u32 = 0x1b;
const APIC_GLOBAL_ENABLE: u64 = 1 << 11;
const APIC_X2APIC_ENABLE: u64 = 1 << 10;
const APIC_BASE_MASK: u64 = 0x000f_ffff_ffff_f000;

const REG_ID: u64 = 0x020;
const REG_TPR: u64 = 0x080;
const REG_EOI: u64 = 0x0b0;
const REG_SVR: u64 = 0x0f0;
const REG_ICR_LOW: u64 = 0x300;
const REG_ICR_HIGH: u64 = 0x310;
const REG_LVT_TIMER: u64 = 0x320;
const REG_LVT_LINT0: u64 = 0x350;
const REG_LVT_LINT1: u64 = 0x360;
const REG_LVT_ERROR: u64 = 0x370;
const REG_TIMER_INITIAL_COUNT: u64 = 0x380;
const REG_TIMER_CURRENT_COUNT: u64 = 0x390;
const REG_TIMER_DIVIDE: u64 = 0x3e0;

const DELIVERY_STATUS_PENDING: u32 = 1 << 12;
const ICR_LEVEL_ASSERT: u32 = 1 << 14;
const ICR_TRIGGER_LEVEL: u32 = 1 << 15;
const DELIVERY_MODE_INIT: u32 = 0b101 << 8;
const DELIVERY_MODE_STARTUP: u32 = 0b110 << 8;
const LVT_MASKED: u32 = 1 << 16;
const LVT_TIMER_PERIODIC: u32 = 1 << 17;
const TIMER_DIVIDE_BY_16: u32 = 0b0011;

static LOCAL_APIC_BASE: AtomicU64 = AtomicU64::new(0);

pub fn initialize(base_from_madt: u64) -> u32 {
    let mut msr = read_msr(IA32_APIC_BASE_MSR);

    if msr & APIC_X2APIC_ENABLE != 0 {
        // Return from x2APIC mode to disabled, then enable ordinary xAPIC MMIO.
        write_msr(
            IA32_APIC_BASE_MSR,
            msr & !(APIC_GLOBAL_ENABLE | APIC_X2APIC_ENABLE),
        );
        msr &= !(APIC_GLOBAL_ENABLE | APIC_X2APIC_ENABLE);
    }

    let requested_base = if base_from_madt != 0 {
        base_from_madt & APIC_BASE_MASK
    } else {
        msr & APIC_BASE_MASK
    };

    write_msr(IA32_APIC_BASE_MSR, requested_base | APIC_GLOBAL_ENABLE);
    LOCAL_APIC_BASE.store(requested_base, Ordering::Release);

    // Disable the legacy PIC. K3 uses the PIT only as a polled calibration
    // reference and does not route its IRQ through the PIC.
    unsafe {
        outb(0x21, 0xff);
        outb(0xa1, 0xff);
    }

    write(REG_TPR, 0);
    write(REG_LVT_TIMER, LVT_MASKED);
    write(REG_TIMER_INITIAL_COUNT, 0);
    write(REG_LVT_LINT0, LVT_MASKED);
    write(REG_LVT_LINT1, LVT_MASKED);
    write(REG_LVT_ERROR, LVT_MASKED);
    write(REG_SVR, (1 << 8) | 0xff);
    write(REG_EOI, 0);

    current_id()
}

pub fn initialize_application_processor() -> u32 {
    let base = LOCAL_APIC_BASE.load(Ordering::Acquire);
    initialize(base)
}

pub fn current_id() -> u32 {
    read(REG_ID) >> 24
}

/// Calibrate a divide-by-16 Local APIC timer against a 10 ms PIT channel-2
/// one-shot. The returned count can be used directly for a 100 Hz periodic
/// scheduler tick.
pub fn calibrate_timer_100hz(timer_vector: u8) -> Result<u32, &'static str> {
    write(REG_TIMER_DIVIDE, TIMER_DIVIDE_BY_16);
    write(REG_LVT_TIMER, LVT_MASKED | timer_vector as u32);

    pit::start_calibration_window();
    write(REG_TIMER_INITIAL_COUNT, u32::MAX);
    pit::wait_for_calibration_window()?;

    let current = read(REG_TIMER_CURRENT_COUNT);
    write(REG_TIMER_INITIAL_COUNT, 0);
    write(REG_LVT_TIMER, LVT_MASKED | timer_vector as u32);

    let elapsed = u32::MAX.wrapping_sub(current);
    if elapsed < 1_000 {
        return Err("Local APIC timer calibration produced an implausibly small count");
    }
    Ok(elapsed)
}

pub fn start_periodic_timer(timer_vector: u8, initial_count: u32) {
    assert!(initial_count != 0, "APIC timer initial count must be nonzero");
    write(REG_TIMER_DIVIDE, TIMER_DIVIDE_BY_16);
    write(REG_LVT_TIMER, LVT_TIMER_PERIODIC | timer_vector as u32);
    write(REG_TIMER_INITIAL_COUNT, initial_count);
}

pub fn mask_timer() {
    let lvt = read(REG_LVT_TIMER);
    write(REG_LVT_TIMER, lvt | LVT_MASKED);
    write(REG_TIMER_INITIAL_COUNT, 0);
}

pub fn end_of_interrupt() {
    write(REG_EOI, 0);
}

pub fn send_init_sipi(apic_id: u32, vector: u8) {
    wait_for_delivery();
    write(REG_ICR_HIGH, apic_id << 24);
    write(
        REG_ICR_LOW,
        DELIVERY_MODE_INIT | ICR_LEVEL_ASSERT | ICR_TRIGGER_LEVEL,
    );
    wait_for_delivery();
    delay(200_000);

    write(REG_ICR_HIGH, apic_id << 24);
    write(REG_ICR_LOW, DELIVERY_MODE_INIT | ICR_TRIGGER_LEVEL);
    wait_for_delivery();
    delay(200_000);

    for _ in 0..2 {
        write(REG_ICR_HIGH, apic_id << 24);
        write(REG_ICR_LOW, DELIVERY_MODE_STARTUP | vector as u32);
        wait_for_delivery();
        delay(200_000);
    }
}

fn wait_for_delivery() {
    for _ in 0..1_000_000 {
        if read(REG_ICR_LOW) & DELIVERY_STATUS_PENDING == 0 {
            return;
        }
        pause();
    }
}

fn delay(iterations: usize) {
    for _ in 0..iterations {
        pause();
    }
}

fn base() -> u64 {
    let base = LOCAL_APIC_BASE.load(Ordering::Acquire);
    assert!(base != 0, "Local APIC used before initialization");
    base
}

fn read(offset: u64) -> u32 {
    // SAFETY: The local APIC MMIO page is identity mapped by the K3 bootstrap
    // tables and the offset is one of the architectural APIC registers.
    unsafe { ptr::read_volatile((base() + offset) as *const u32) }
}

fn write(offset: u64, value: u32) {
    // SAFETY: The local APIC MMIO page is identity mapped by the K3 bootstrap
    // tables and the offset is one of the architectural APIC registers.
    unsafe { ptr::write_volatile((base() + offset) as *mut u32, value) }
}
