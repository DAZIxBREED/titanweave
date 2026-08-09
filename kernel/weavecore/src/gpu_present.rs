//! K13.C compositor-presentation policy.
//!
//! The hardware backend owns queue mechanics; this module owns backend-neutral
//! buffering, damage validation, fence sequencing, pacing metadata, and the
//! watchdog/fallback decision used by DISPLAYD-mediated presents.

pub const PRESENT_BUFFER_COUNT: usize = 3;
pub const MAX_IN_FLIGHT_FRAMES: u32 = 2;
pub const DEFAULT_REFRESH_MILLIHZ: u32 = 60_000;
pub const PRESENT_STALL_LIMIT: u32 = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DamageRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl DamageRect {
    pub const FULL: Self = Self { x: 0, y: 0, width: u32::MAX, height: u32::MAX };

    #[must_use]
    pub const fn is_empty(self) -> bool { self.width == 0 || self.height == 0 }

    pub fn clipped(self, display_width: u32, display_height: u32) -> Result<Self, &'static str> {
        if display_width == 0 || display_height == 0 {
            return Err("presentation target has no dimensions");
        }
        if self == Self::FULL {
            return Ok(Self { x: 0, y: 0, width: display_width, height: display_height });
        }
        if self.is_empty() || self.x >= display_width || self.y >= display_height {
            return Err("damage rectangle lies outside scanout");
        }
        let right = self.x.saturating_add(self.width).min(display_width);
        let bottom = self.y.saturating_add(self.height).min(display_height);
        if right <= self.x || bottom <= self.y {
            return Err("damage rectangle clips to empty");
        }
        Ok(Self { x: self.x, y: self.y, width: right - self.x, height: bottom - self.y })
    }

    #[must_use]
    pub fn byte_offset(self, stride_pixels: u32) -> Option<u64> {
        u64::from(self.y)
            .checked_mul(u64::from(stride_pixels))?
            .checked_add(u64::from(self.x))?
            .checked_mul(4)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FramePacer {
    refresh_millihz: u32,
    period_ns: u64,
    next_deadline_ns: u64,
}

impl FramePacer {
    pub fn new(refresh_millihz: u32) -> Result<Self, &'static str> {
        if !(10_000..=1_000_000).contains(&refresh_millihz) {
            return Err("refresh rate is outside compositor pacing bounds");
        }
        let period_ns = 1_000_000_000_000u64 / u64::from(refresh_millihz);
        Ok(Self { refresh_millihz, period_ns, next_deadline_ns: period_ns })
    }

    pub fn advance(&mut self) -> u64 {
        let deadline = self.next_deadline_ns;
        self.next_deadline_ns = self.next_deadline_ns.saturating_add(self.period_ns);
        deadline
    }

    #[must_use] pub const fn refresh_millihz(self) -> u32 { self.refresh_millihz }
    #[must_use] pub const fn period_ns(self) -> u64 { self.period_ns }
}

#[derive(Clone, Copy, Debug)]
pub struct PresentWatchdog {
    consecutive_stalls: u32,
    fallback_armed: bool,
}

impl PresentWatchdog {
    pub const fn new() -> Self { Self { consecutive_stalls: 0, fallback_armed: true } }

    pub fn completion(&mut self) { self.consecutive_stalls = 0; }

    pub fn stall(&mut self) -> bool {
        self.consecutive_stalls = self.consecutive_stalls.saturating_add(1);
        self.consecutive_stalls >= PRESENT_STALL_LIMIT
    }

    #[must_use] pub const fn fallback_armed(self) -> bool { self.fallback_armed }
    #[must_use] pub const fn consecutive_stalls(self) -> u32 { self.consecutive_stalls }
}

#[derive(Clone, Copy, Debug)]
pub struct PresentPolicyReport {
    pub buffers: usize,
    pub max_in_flight: u32,
    pub refresh_millihz: u32,
    pub period_ns: u64,
    pub fallback_after_stalls: u32,
    pub damage_offset: u64,
}

pub fn run_self_test() -> Result<PresentPolicyReport, &'static str> {
    let damage = DamageRect { x: 32, y: 16, width: 128, height: 64 }.clipped(1024, 768)?;
    let damage_offset = damage.byte_offset(1024).ok_or("damage byte offset overflow")?;
    if damage_offset != ((16u64 * 1024 + 32) * 4) {
        return Err("damage byte-offset self-test failed");
    }

    let mut pacer = FramePacer::new(DEFAULT_REFRESH_MILLIHZ)?;
    let first = pacer.advance();
    let second = pacer.advance();
    if second <= first || second - first != pacer.period_ns() {
        return Err("frame pacing self-test failed");
    }

    let mut watchdog = PresentWatchdog::new();
    if watchdog.stall() || watchdog.stall() || !watchdog.stall() {
        return Err("presentation fallback threshold self-test failed");
    }
    watchdog.completion();
    if watchdog.consecutive_stalls() != 0 || !watchdog.fallback_armed() {
        return Err("presentation watchdog reset self-test failed");
    }

    Ok(PresentPolicyReport {
        buffers: PRESENT_BUFFER_COUNT,
        max_in_flight: MAX_IN_FLIGHT_FRAMES,
        refresh_millihz: pacer.refresh_millihz(),
        period_ns: pacer.period_ns(),
        fallback_after_stalls: PRESENT_STALL_LIMIT,
        damage_offset,
    })
}
