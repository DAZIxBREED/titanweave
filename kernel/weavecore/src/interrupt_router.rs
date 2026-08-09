//! K11 interrupt-vector ownership, dispatch accounting, masking, routing and handlers.
use crate::abi::SYSCALL_VECTOR;
use crate::device::DeviceId;

pub const FIRST_DEVICE_VECTOR: u8 = 0x50;
pub const LAST_DEVICE_VECTOR: u8 = 0xdf;
const DEVICE_VECTOR_SPAN: usize = (LAST_DEVICE_VECTOR - FIRST_DEVICE_VECTOR + 1) as usize;
pub const MAX_INTERRUPT_ROUTES: usize = DEVICE_VECTOR_SPAN - 1;
pub type DeviceInterruptHandler = fn(u8, DeviceId) -> Result<(), &'static str>;

#[derive(Clone, Copy)]
pub struct InterruptRoute {
    pub vector: u8,
    pub device: DeviceId,
    pub cpu: u16,
    pub shared: bool,
    pub active: bool,
    pub masked: bool,
    pub dispatches: u64,
    pub spurious: u64,
    pub handler: Option<DeviceInterruptHandler>,
}
impl InterruptRoute {
    pub const EMPTY: Self = Self {
        vector: 0, device: DeviceId(0), cpu: 0, shared: false, active: false,
        masked: true, dispatches: 0, spurious: 0, handler: None,
    };
}

pub struct InterruptRouter { routes: [InterruptRoute; MAX_INTERRUPT_ROUTES] }
impl InterruptRouter {
    pub const fn new() -> Self { Self { routes: [InterruptRoute::EMPTY; MAX_INTERRUPT_ROUTES] } }

    pub fn allocate(&mut self, device: DeviceId, cpu: u16, shared: bool) -> Result<InterruptRoute, &'static str> {
        if device.0 == 0 { return Err("invalid interrupt owner"); }
        let (i, slot) = self.routes.iter_mut().enumerate().find(|(_, r)| !r.active)
            .ok_or("interrupt vectors exhausted")?;
        let raw_vector = FIRST_DEVICE_VECTOR + i as u8;
        let vector = if raw_vector >= SYSCALL_VECTOR {
            raw_vector.checked_add(1).ok_or("interrupt vector overflow")?
        } else {
            raw_vector
        };
        if vector > LAST_DEVICE_VECTOR || vector == SYSCALL_VECTOR {
            return Err("interrupt vector reservation failure");
        }
        let route = InterruptRoute {
            vector, device, cpu, shared, active: true,
            masked: true, dispatches: 0, spurious: 0, handler: None,
        };
        *slot = route;
        Ok(route)
    }

    pub fn register_handler(&mut self, vector: u8, device: DeviceId, handler: DeviceInterruptHandler) -> Result<(), &'static str> {
        let route = self.route_mut(vector, device)?;
        if !route.masked { return Err("interrupt must be masked while installing handler"); }
        route.handler = Some(handler);
        Ok(())
    }

    pub fn enable(&mut self, vector: u8, device: DeviceId) -> Result<(), &'static str> {
        let route = self.route_mut(vector, device)?;
        if route.handler.is_none() { return Err("interrupt has no registered handler"); }
        route.masked = false;
        Ok(())
    }
    pub fn mask(&mut self, vector: u8, device: DeviceId) -> Result<(), &'static str> {
        self.route_mut(vector, device)?.masked = true;
        Ok(())
    }
    pub fn release(&mut self, vector: u8, device: DeviceId) -> Result<(), &'static str> {
        let route = self.route_mut(vector, device)?;
        route.active = false;
        route.masked = true;
        route.handler = None;
        Ok(())
    }

    pub fn record_dispatch(&mut self, vector: u8) -> Result<DeviceId, &'static str> {
        let route = self.routes.iter_mut().find(|r| r.active && r.vector == vector)
            .ok_or("unowned interrupt vector")?;
        if route.masked {
            route.spurious = route.spurious.saturating_add(1);
            return Err("interrupt arrived while masked");
        }
        if route.handler.is_none() {
            route.spurious = route.spurious.saturating_add(1);
            return Err("interrupt has no handler");
        }
        route.dispatches = route.dispatches.saturating_add(1);
        Ok(route.device)
    }

    pub fn handler(&self, vector: u8, device: DeviceId) -> Option<DeviceInterruptHandler> {
        self.routes.iter().find(|r| r.active && r.vector == vector && r.device == device)
            .and_then(|r| r.handler)
    }
    pub fn record_spurious(&mut self, vector: u8) {
        if let Some(route) = self.routes.iter_mut().find(|r| r.active && r.vector == vector) {
            route.spurious = route.spurious.saturating_add(1);
        }
    }
    pub fn owner(&self, vector: u8) -> Option<DeviceId> {
        self.routes.iter().find(|r| r.active && r.vector == vector).map(|r| r.device)
    }
    pub fn migrate(&mut self, vector: u8, device: DeviceId, cpu: u16) -> Result<(), &'static str> {
        let route = self.route_mut(vector, device)?;
        if !route.masked { return Err("interrupt must be masked before migration"); }
        route.cpu = cpu;
        Ok(())
    }
    fn route_mut(&mut self, vector: u8, device: DeviceId) -> Result<&mut InterruptRoute, &'static str> {
        self.routes.iter_mut().find(|r| r.active && r.vector == vector && r.device == device)
            .ok_or("interrupt route not found")
    }
}
