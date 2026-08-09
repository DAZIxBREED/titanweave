//! K12 display bootstrap and qualification state.

use crate::{compositor, forgegraphics, framebuffer::Framebuffer, input_router, serial, workplace_shell};
use crate::sync::SpinLock;
use titanweave_boot_protocol::BootInfo;

#[derive(Clone, Copy, Debug)]
pub struct DisplayState {
    pub available: bool,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixel_format: u32,
    pub firmware_fallback: bool,
}

impl DisplayState {
    pub const EMPTY: Self = Self {
        available: false,
        width: 0,
        height: 0,
        stride: 0,
        pixel_format: 0,
        firmware_fallback: false,
    };
}

static STATE: SpinLock<DisplayState> = SpinLock::new(DisplayState::EMPTY);

pub fn initialize(boot_info: &BootInfo) -> Result<DisplayState, &'static str> {
    let mut framebuffer = Framebuffer::from_boot_info(boot_info)?;
    let info = framebuffer.info();
    framebuffer.draw_boot_card();

    let compositor_report = compositor::run_self_test(info.width, info.height)?;
    serial::println(format_args!(
        "[COMP] surface/damage self-test: surfaces={} damage={} hit={}",
        compositor_report.surfaces,
        compositor_report.damage_rects,
        compositor_report.hit_surface
    ));

    let input_report = input_router::run_self_test(info.width, info.height)?;
    serial::println(format_args!(
        "[INPT] focus/capture self-test: events={} target={} pointer={},{}",
        input_report.events,
        input_report.final_target,
        input_report.pointer_x,
        input_report.pointer_y
    ));

    let adapters = forgegraphics::run_self_test()?;
    serial::println(format_args!(
        "[FGFX] ForgeGraphics ABI v{} backend contract passed adapters={}",
        forgegraphics::FORGEGRAPHICS_ABI_VERSION,
        adapters
    ));

    let preview = workplace_shell::render_preview(&mut framebuffer);
    serial::println(format_args!(
        "[WPS ] Workplace Shell reference preview rendered: windows={} navigator_rows={} task_buttons={}",
        preview.windows,
        preview.navigator_rows,
        preview.task_buttons
    ));

    let state = DisplayState {
        available: true,
        width: info.width,
        height: info.height,
        stride: info.stride,
        pixel_format: info.pixel_format,
        firmware_fallback: true,
    };
    *STATE.lock() = state;
    serial::println(format_args!(
        "[GFX ] K13 GOP scanout online: {}x{} stride={} format={} fallback=firmware",
        state.width, state.height, state.stride, state.pixel_format
    ));
    Ok(state)
}

pub fn initialize_headless() -> DisplayState {
    let state = DisplayState::EMPTY;
    *STATE.lock() = state;
    serial::println(format_args!(
        "[GFX ] K13 running headless: no usable GOP linear framebuffer"
    ));
    state
}

#[must_use]
pub fn state() -> DisplayState { *STATE.lock() }

/// Compact v1 userspace query: high 32 bits = width, low 32 bits = height.
/// Zero means no active display.  The richer C-layout ABI lives in graphics_abi.
#[must_use]
pub fn packed_primary_mode() -> u64 {
    let state = state();
    if !state.available { 0 } else { ((state.width as u64) << 32) | state.height as u64 }
}


pub fn log_primary() {
    let state = state();
    if state.available {
        serial::println(format_args!(
            "[SHELL] display: {}x{} stride={} format={} backend=firmware-gop",
            state.width, state.height, state.stride, state.pixel_format
        ));
    } else {
        serial::println(format_args!("[SHELL] display: headless"));
    }
}
