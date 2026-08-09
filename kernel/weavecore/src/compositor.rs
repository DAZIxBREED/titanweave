//! K12/K13 software compositor policy and surface lifecycle.
//!
//! This module does not make the kernel the permanent desktop compositor.
//! Instead it defines the safety-critical surface bookkeeping, damage clipping,
//! focus/hit-test semantics, and fallback composition rules DISPLAYD can rely on.

pub type SurfaceId = u64;
pub type ProcessId = u64;

pub const MAX_SURFACES: usize = 64;
pub const MAX_DAMAGE_RECTS: usize = 64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const EMPTY: Self = Self { x: 0, y: 0, width: 0, height: 0 };

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    #[must_use]
    pub fn contains(self, x: i32, y: i32) -> bool {
        if self.is_empty() || x < self.x || y < self.y {
            return false;
        }
        let right = i64::from(self.x) + i64::from(self.width);
        let bottom = i64::from(self.y) + i64::from(self.height);
        i64::from(x) < right && i64::from(y) < bottom
    }

    #[must_use]
    pub fn clipped_to(self, width: u32, height: u32) -> Self {
        if self.is_empty() {
            return Self::EMPTY;
        }
        let x0 = i64::from(self.x).clamp(0, i64::from(width));
        let y0 = i64::from(self.y).clamp(0, i64::from(height));
        let x1 = (i64::from(self.x) + i64::from(self.width)).clamp(0, i64::from(width));
        let y1 = (i64::from(self.y) + i64::from(self.height)).clamp(0, i64::from(height));
        if x1 <= x0 || y1 <= y0 {
            Self::EMPTY
        } else {
            Self {
                x: x0 as i32,
                y: y0 as i32,
                width: (x1 - x0) as u32,
                height: (y1 - y0) as u32,
            }
        }
    }

    #[must_use]
    pub fn union(self, other: Self) -> Self {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        let x0 = i64::from(self.x).min(i64::from(other.x));
        let y0 = i64::from(self.y).min(i64::from(other.y));
        let x1 = (i64::from(self.x) + i64::from(self.width))
            .max(i64::from(other.x) + i64::from(other.width));
        let y1 = (i64::from(self.y) + i64::from(self.height))
            .max(i64::from(other.y) + i64::from(other.height));
        Self {
            x: x0 as i32,
            y: y0 as i32,
            width: (x1 - x0) as u32,
            height: (y1 - y0) as u32,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Surface {
    occupied: bool,
    id: SurfaceId,
    owner: ProcessId,
    bounds: Rect,
    z: i32,
    opacity: u8,
    visible: bool,
    generation: u32,
}

impl Surface {
    const EMPTY: Self = Self {
        occupied: false,
        id: 0,
        owner: 0,
        bounds: Rect::EMPTY,
        z: 0,
        opacity: 0,
        visible: false,
        generation: 0,
    };
}

#[derive(Clone, Copy)]
pub struct DamageTracker {
    rects: [Rect; MAX_DAMAGE_RECTS],
    count: usize,
    full: bool,
    bounds: Rect,
}

impl DamageTracker {
    pub const fn new() -> Self {
        Self {
            rects: [Rect::EMPTY; MAX_DAMAGE_RECTS],
            count: 0,
            full: false,
            bounds: Rect::EMPTY,
        }
    }

    pub fn add(&mut self, rect: Rect, display_width: u32, display_height: u32) {
        let clipped = rect.clipped_to(display_width, display_height);
        if clipped.is_empty() {
            return;
        }
        self.bounds = self.bounds.union(clipped);
        if self.full {
            return;
        }
        if self.count == MAX_DAMAGE_RECTS {
            self.full = true;
            self.count = 1;
            self.rects[0] = self.bounds;
            return;
        }
        self.rects[self.count] = clipped;
        self.count += 1;
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }

    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }

    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.full
    }

    #[must_use]
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
}

pub struct SurfaceRegistry {
    surfaces: [Surface; MAX_SURFACES],
    next_id: SurfaceId,
    display_width: u32,
    display_height: u32,
    damage: DamageTracker,
}

impl SurfaceRegistry {
    pub const fn new(display_width: u32, display_height: u32) -> Self {
        Self {
            surfaces: [Surface::EMPTY; MAX_SURFACES],
            next_id: 1,
            display_width,
            display_height,
            damage: DamageTracker::new(),
        }
    }

    pub fn create(
        &mut self,
        owner: ProcessId,
        bounds: Rect,
        z: i32,
        opacity: u8,
    ) -> Result<SurfaceId, &'static str> {
        if owner == 0 || bounds.width == 0 || bounds.height == 0 {
            return Err("invalid surface creation request");
        }
        let clipped = bounds.clipped_to(self.display_width, self.display_height);
        if clipped.is_empty() {
            return Err("surface lies outside the display");
        }
        let slot = self
            .surfaces
            .iter_mut()
            .find(|surface| !surface.occupied)
            .ok_or("surface table is full")?;
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or("surface id overflow")?;
        *slot = Surface {
            occupied: true,
            id,
            owner,
            bounds: clipped,
            z,
            opacity,
            visible: true,
            generation: 1,
        };
        self.damage.add(clipped, self.display_width, self.display_height);
        Ok(id)
    }

    pub fn move_surface(&mut self, owner: ProcessId, id: SurfaceId, x: i32, y: i32) -> Result<(), &'static str> {
        let index = self.surface_index(owner, id)?;
        let old = self.surfaces[index].bounds;
        let candidate = Rect { x, y, ..old }.clipped_to(self.display_width, self.display_height);
        if candidate.is_empty() {
            return Err("surface move leaves the display");
        }
        self.surfaces[index].bounds = candidate;
        self.surfaces[index].generation = self.surfaces[index].generation.saturating_add(1);
        self.damage.add(old, self.display_width, self.display_height);
        self.damage.add(candidate, self.display_width, self.display_height);
        Ok(())
    }

    pub fn set_z(&mut self, owner: ProcessId, id: SurfaceId, z: i32) -> Result<(), &'static str> {
        let index = self.surface_index(owner, id)?;
        self.surfaces[index].z = z;
        self.surfaces[index].generation = self.surfaces[index].generation.saturating_add(1);
        self.damage.add(self.surfaces[index].bounds, self.display_width, self.display_height);
        Ok(())
    }

    pub fn present(&mut self, owner: ProcessId, id: SurfaceId, local_damage: Rect) -> Result<(), &'static str> {
        let index = self.surface_index(owner, id)?;
        let surface = self.surfaces[index];
        let translated = Rect {
            x: surface.bounds.x.saturating_add(local_damage.x),
            y: surface.bounds.y.saturating_add(local_damage.y),
            width: local_damage.width.min(surface.bounds.width),
            height: local_damage.height.min(surface.bounds.height),
        };
        self.damage.add(translated, self.display_width, self.display_height);
        Ok(())
    }

    pub fn destroy(&mut self, owner: ProcessId, id: SurfaceId) -> Result<(), &'static str> {
        let index = self.surface_index(owner, id)?;
        let old = self.surfaces[index].bounds;
        self.surfaces[index] = Surface::EMPTY;
        self.damage.add(old, self.display_width, self.display_height);
        Ok(())
    }

    #[must_use]
    pub fn hit_test(&self, x: i32, y: i32) -> Option<SurfaceId> {
        self.surfaces
            .iter()
            .filter(|surface| surface.occupied && surface.visible && surface.opacity != 0 && surface.bounds.contains(x, y))
            .max_by_key(|surface| (surface.z, surface.id))
            .map(|surface| surface.id)
    }

    #[must_use]
    pub fn damage(&self) -> &DamageTracker {
        &self.damage
    }

    pub fn clear_damage(&mut self) {
        self.damage.clear();
    }

    fn surface_index(&self, owner: ProcessId, id: SurfaceId) -> Result<usize, &'static str> {
        self.surfaces
            .iter()
            .position(|surface| surface.occupied && surface.id == id && surface.owner == owner)
            .ok_or("surface not found or owned by another process")
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CompositorSelfTestReport {
    pub surfaces: usize,
    pub damage_rects: usize,
    pub hit_surface: SurfaceId,
}

pub fn run_self_test(display_width: u32, display_height: u32) -> Result<CompositorSelfTestReport, &'static str> {
    if display_width < 320 || display_height < 200 {
        return Err("display is too small for compositor self-test");
    }
    let mut registry = SurfaceRegistry::new(display_width, display_height);
    let first = registry.create(
        10,
        Rect { x: 16, y: 16, width: 160, height: 120 },
        1,
        255,
    )?;
    let second = registry.create(
        11,
        Rect { x: 64, y: 48, width: 160, height: 120 },
        2,
        224,
    )?;
    if registry.hit_test(80, 64) != Some(second) {
        return Err("z-order hit testing failed");
    }
    registry.move_surface(10, first, 24, 24)?;
    registry.present(11, second, Rect { x: 0, y: 0, width: 32, height: 32 })?;
    if registry.damage().count() == 0 || registry.damage().bounds().is_empty() {
        return Err("damage tracking failed");
    }
    let damage_rects = registry.damage().count();
    registry.destroy(10, first)?;
    Ok(CompositorSelfTestReport {
        surfaces: 2,
        damage_rects,
        hit_surface: second,
    })
}
