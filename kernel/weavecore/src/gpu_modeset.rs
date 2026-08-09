//! K13 atomic modeset/scanout contract.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh_millihz: u32,
    pub pixel_clock_khz: u32,
}

impl DisplayMode {
    #[must_use]
    pub const fn is_sane(self) -> bool {
        self.width >= 320 && self.height >= 200
            && self.width <= 16384 && self.height <= 16384
            && self.refresh_millihz >= 10_000 && self.refresh_millihz <= 1_000_000
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AtomicModeRequest {
    pub connector_id: u64,
    pub scanout_buffer_id: u64,
    pub mode: DisplayMode,
    pub enable_vrr: bool,
    pub enable_hdr: bool,
}

impl AtomicModeRequest {
    pub fn validate(self) -> Result<(), &'static str> {
        if self.connector_id == 0 || self.scanout_buffer_id == 0 { return Err("invalid modeset object id"); }
        if !self.mode.is_sane() { return Err("invalid display mode"); }
        // K13 groundwork records VRR/HDR intent but does not claim those paths
        // operational until a backend reports the required capability bits.
        Ok(())
    }
}

pub fn run_self_test() -> Result<DisplayMode, &'static str> {
    let mode = DisplayMode { width: 2560, height: 1440, refresh_millihz: 144_000, pixel_clock_khz: 0 };
    AtomicModeRequest {
        connector_id: 1,
        scanout_buffer_id: 7,
        mode,
        enable_vrr: false,
        enable_hdr: false,
    }.validate()?;
    Ok(mode)
}
