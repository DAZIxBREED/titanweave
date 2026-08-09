//! K12/K13 stable graphics ABI declarations.
//!
//! The kernel owns memory protection, scanout device arbitration, and capability
//! checks. The long-term compositor lives in DISPLAYD and talks to GPU drivers
//! through ForgeBus. These C-layout structures are intentionally backend-neutral
//! so firmware GOP, VirtIO-GPU, AMD, Intel, and NVIDIA scanout paths can share the
//! same userspace contract.

pub const GRAPHICS_ABI_VERSION: u32 = 1;
pub const MAX_DISPLAYS: usize = 16;
pub const MAX_SURFACES: usize = 256;

pub const SURFACE_FLAG_VISIBLE: u32 = 1 << 0;
pub const SURFACE_FLAG_OPAQUE: u32 = 1 << 1;
pub const SURFACE_FLAG_CURSOR: u32 = 1 << 2;
pub const SURFACE_FLAG_PROTECTED: u32 = 1 << 3;

pub const PRESENT_FLAG_VSYNC: u32 = 1 << 0;
pub const PRESENT_FLAG_IMMEDIATE: u32 = 1 << 1;
pub const PRESENT_FLAG_ALLOW_TEARING: u32 = 1 << 2;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DisplayInfo {
    pub display_id: u64,
    pub width: u32,
    pub height: u32,
    pub refresh_millihz: u32,
    pub scale_milli: u32,
    pub pixel_format: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurfaceCreate {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentRequest {
    pub surface_id: u64,
    pub display_id: u64,
    pub damage_x: i32,
    pub damage_y: i32,
    pub damage_width: u32,
    pub damage_height: u32,
    pub flags: u32,
    pub reserved: u32,
    /// Reserved for ForgeGraphics v1 extension without changing the ABI size.
    pub reserved2: u64,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEventKind {
    KeyDown = 1,
    KeyUp = 2,
    PointerMove = 3,
    PointerButtonDown = 4,
    PointerButtonUp = 5,
    Wheel = 6,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputEvent {
    pub sequence: u64,
    pub timestamp_ticks: u64,
    pub target_surface: u64,
    pub kind: u32,
    pub code: u32,
    pub value_x: i32,
    pub value_y: i32,
}

const _: [(); 32] = [(); core::mem::size_of::<DisplayInfo>()];
const _: [(); 16] = [(); core::mem::size_of::<SurfaceCreate>()];
const _: [(); 48] = [(); core::mem::size_of::<PresentRequest>()];
const _: [(); 40] = [(); core::mem::size_of::<InputEvent>()];
