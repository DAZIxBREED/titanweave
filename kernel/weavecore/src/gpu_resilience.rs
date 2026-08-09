//! K13.D GPU resilience, hot-plug, scanout and failover policy.
//!
//! This layer is deliberately backend-neutral. Native AMD/Intel/NVIDIA and
//! VirtIO backends feed the same health/failover state machine. K13.D runtime
//! qualification combines these deterministic policy tests with a live
//! VirtIO-GPU presentation soak/recovery cycle in `gpu_runtime`.

use crate::{
    device::DeviceId,
    forgegraphics::{CAP_MULTI_GPU_COPY, CAP_SHARED_MEMORY},
    gpu_multigpu::{self, TransferRoute},
    pci_address::PciAddress,
    pcie_hotplug::HotplugController,
};

pub const MAX_RESILIENCE_ADAPTERS: usize = 8;
pub const MAX_MANAGED_SCANOUTS: usize = 4;
pub const GPU_STALL_RECOVERY_THRESHOLD: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterHealthState {
    Empty,
    Healthy,
    Degraded,
    Resetting,
    Rebinding,
    Offline,
    Quarantined,
}

#[derive(Clone, Copy, Debug)]
pub struct AdapterHealth {
    pub id: DeviceId,
    pub state: AdapterHealthState,
    pub generation: u32,
    pub stalls: u32,
    pub can_present: bool,
}

impl AdapterHealth {
    const EMPTY: Self = Self {
        id: DeviceId(0),
        state: AdapterHealthState::Empty,
        generation: 0,
        stalls: 0,
        can_present: false,
    };
}

pub struct GpuHealthManager {
    adapters: [AdapterHealth; MAX_RESILIENCE_ADAPTERS],
    count: usize,
    primary: Option<usize>,
    fallback_armed: bool,
    failovers: u32,
    recoveries: u32,
}

impl GpuHealthManager {
    pub const fn new() -> Self {
        Self {
            adapters: [AdapterHealth::EMPTY; MAX_RESILIENCE_ADAPTERS],
            count: 0,
            primary: None,
            fallback_armed: true,
            failovers: 0,
            recoveries: 0,
        }
    }

    pub fn register(&mut self, id: DeviceId, can_present: bool) -> Result<(), &'static str> {
        if id.0 == 0 {
            return Err("GPU health manager rejected zero device id");
        }
        if self.adapters[..self.count].iter().any(|adapter| adapter.id == id) {
            return Err("GPU health manager rejected duplicate device");
        }
        if self.count >= MAX_RESILIENCE_ADAPTERS {
            return Err("GPU health manager is full");
        }
        let index = self.count;
        self.adapters[index] = AdapterHealth {
            id,
            state: AdapterHealthState::Healthy,
            generation: 1,
            stalls: 0,
            can_present,
        };
        self.count += 1;
        if self.primary.is_none() && can_present {
            self.primary = Some(index);
        }
        Ok(())
    }

    pub fn record_success(&mut self, id: DeviceId) -> Result<(), &'static str> {
        let adapter = self.find_mut(id)?;
        adapter.stalls = 0;
        if matches!(adapter.state, AdapterHealthState::Degraded) {
            adapter.state = AdapterHealthState::Healthy;
        }
        Ok(())
    }

    /// Returns true when the configured recovery threshold has been reached.
    pub fn record_stall(&mut self, id: DeviceId) -> Result<bool, &'static str> {
        let adapter = self.find_mut(id)?;
        if !matches!(adapter.state, AdapterHealthState::Healthy | AdapterHealthState::Degraded) {
            return Err("GPU stall reported in a non-running state");
        }
        adapter.stalls = adapter.stalls.saturating_add(1);
        adapter.state = AdapterHealthState::Degraded;
        Ok(adapter.stalls >= GPU_STALL_RECOVERY_THRESHOLD)
    }

    pub fn begin_recovery(&mut self, id: DeviceId) -> Result<(), &'static str> {
        let adapter = self.find_mut(id)?;
        adapter.state = AdapterHealthState::Resetting;
        adapter.generation = adapter.generation.wrapping_add(1).max(1);
        Ok(())
    }

    pub fn mark_rebinding(&mut self, id: DeviceId) -> Result<(), &'static str> {
        let adapter = self.find_mut(id)?;
        if adapter.state != AdapterHealthState::Resetting {
            return Err("GPU rebind attempted before reset state");
        }
        adapter.state = AdapterHealthState::Rebinding;
        Ok(())
    }

    pub fn complete_rebind(&mut self, id: DeviceId) -> Result<(), &'static str> {
        let index = self.index_of(id).ok_or("GPU health manager device not found")?;
        if self.adapters[index].state != AdapterHealthState::Rebinding {
            return Err("GPU recovery completed outside rebind state");
        }
        self.adapters[index].state = AdapterHealthState::Healthy;
        self.adapters[index].stalls = 0;
        let can_present = self.adapters[index].can_present;
        self.recoveries = self.recoveries.saturating_add(1);
        if self.primary.is_none() && can_present {
            self.primary = Some(index);
        }
        Ok(())
    }

    pub fn surprise_remove(&mut self, id: DeviceId) -> Result<Option<DeviceId>, &'static str> {
        let removed_index = self.index_of(id).ok_or("GPU removal referenced unknown device")?;
        self.adapters[removed_index].state = AdapterHealthState::Offline;
        self.adapters[removed_index].generation = self.adapters[removed_index].generation.wrapping_add(1).max(1);

        if self.primary == Some(removed_index) {
            self.primary = None;
            for index in 0..self.count {
                if index != removed_index
                    && self.adapters[index].can_present
                    && self.adapters[index].state == AdapterHealthState::Healthy
                {
                    self.primary = Some(index);
                    break;
                }
            }
            self.failovers = self.failovers.saturating_add(1);
        }
        Ok(self.primary.map(|index| self.adapters[index].id))
    }

    pub fn quarantine(&mut self, id: DeviceId) -> Result<(), &'static str> {
        let index = self.index_of(id).ok_or("GPU health manager device not found")?;
        self.adapters[index].state = AdapterHealthState::Quarantined;
        self.adapters[index].can_present = false;
        if self.primary == Some(index) {
            self.primary = None;
        }
        Ok(())
    }

    #[must_use]
    pub fn primary(&self) -> Option<DeviceId> {
        self.primary.map(|index| self.adapters[index].id)
    }

    #[must_use]
    pub const fn fallback_armed(&self) -> bool { self.fallback_armed }
    #[must_use]
    pub const fn failovers(&self) -> u32 { self.failovers }
    #[must_use]
    pub const fn recoveries(&self) -> u32 { self.recoveries }

    fn index_of(&self, id: DeviceId) -> Option<usize> {
        self.adapters[..self.count].iter().position(|adapter| adapter.id == id)
    }

    fn find_mut(&mut self, id: DeviceId) -> Result<&mut AdapterHealth, &'static str> {
        self.adapters[..self.count]
            .iter_mut()
            .find(|adapter| adapter.id == id)
            .ok_or("GPU health manager device not found")
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ScanoutTopology {
    connected: [bool; MAX_MANAGED_SCANOUTS],
    primary: Option<usize>,
    generation: u32,
}

impl ScanoutTopology {
    pub const fn new() -> Self {
        Self { connected: [false; MAX_MANAGED_SCANOUTS], primary: None, generation: 1 }
    }

    pub fn connect(&mut self, scanout: usize) -> Result<(), &'static str> {
        if scanout >= MAX_MANAGED_SCANOUTS {
            return Err("scanout id exceeds K13.D managed display bound");
        }
        self.connected[scanout] = true;
        self.generation = self.generation.wrapping_add(1).max(1);
        if self.primary.is_none() {
            self.primary = Some(scanout);
        }
        Ok(())
    }

    pub fn disconnect(&mut self, scanout: usize) -> Result<Option<usize>, &'static str> {
        if scanout >= MAX_MANAGED_SCANOUTS || !self.connected[scanout] {
            return Err("scanout disconnect referenced inactive output");
        }
        self.connected[scanout] = false;
        self.generation = self.generation.wrapping_add(1).max(1);
        if self.primary == Some(scanout) {
            self.primary = self.connected.iter().position(|connected| *connected);
        }
        Ok(self.primary)
    }

    #[must_use]
    pub fn connected_count(&self) -> usize {
        self.connected.iter().filter(|connected| **connected).count()
    }

    #[must_use]
    pub const fn primary(&self) -> Option<usize> { self.primary }
    #[must_use]
    pub const fn generation(&self) -> u32 { self.generation }
}

#[derive(Clone, Copy, Debug)]
pub struct GpuResilienceReport {
    pub recovery_threshold: u32,
    pub recoveries: u32,
    pub failovers: u32,
    pub hotplug_events: u32,
    pub managed_scanouts: usize,
    pub promoted_scanout: usize,
    pub transfer_route: TransferRoute,
    pub fallback_armed: bool,
}

pub fn run_self_test() -> Result<GpuResilienceReport, &'static str> {
    let primary = DeviceId(0x13d1);
    let standby = DeviceId(0x13d2);
    let mut health = GpuHealthManager::new();
    health.register(primary, true)?;
    health.register(standby, true)?;
    if health.primary() != Some(primary) || !health.fallback_armed() {
        return Err("GPU health primary/fallback initialization failed");
    }
    if health.record_stall(primary)? || health.record_stall(primary)? || !health.record_stall(primary)? {
        return Err("GPU stall recovery threshold self-test failed");
    }
    health.begin_recovery(primary)?;
    health.mark_rebinding(primary)?;
    health.complete_rebind(primary)?;
    health.record_success(primary)?;
    if health.recoveries() != 1 || health.primary() != Some(primary) {
        return Err("GPU rebind recovery self-test failed");
    }
    let promoted = health.surprise_remove(primary)?;
    if promoted != Some(standby) || health.failovers() != 1 {
        return Err("GPU standby promotion self-test failed");
    }

    let mut outputs = ScanoutTopology::new();
    outputs.connect(0)?;
    outputs.connect(1)?;
    if outputs.connected_count() != 2 || outputs.primary() != Some(0) {
        return Err("multi-scanout connection self-test failed");
    }
    let promoted_scanout = outputs.disconnect(0)?.ok_or("multi-scanout primary promotion failed")?;
    if promoted_scanout != 1 || outputs.generation() < 4 {
        return Err("multi-scanout generation/promotion self-test failed");
    }

    let route = gpu_multigpu::choose_route(
        primary.0,
        standby.0,
        CAP_MULTI_GPU_COPY | CAP_SHARED_MEMORY,
        CAP_MULTI_GPU_COPY | CAP_SHARED_MEMORY,
    );
    if route != TransferRoute::PeerToPeer {
        return Err("K13.D multi-GPU route policy self-test failed");
    }

    let bridge = PciAddress { segment: 0, bus: 0, device: 1, function: 0 };
    let mut hotplug = HotplugController::new();
    hotplug.register_slot(bridge, 1)?;
    hotplug.presence_change(bridge, 1, true, 100)?;
    let mut arrived = 0u32;
    hotplug.poll(111, |_, _, _| arrived = arrived.saturating_add(1), |_, _| {})?;
    if arrived != 1 {
        return Err("GPU hot-plug arrival debounce self-test failed");
    }

    Ok(GpuResilienceReport {
        recovery_threshold: GPU_STALL_RECOVERY_THRESHOLD,
        recoveries: health.recoveries(),
        failovers: health.failovers(),
        hotplug_events: arrived,
        managed_scanouts: outputs.connected_count(),
        promoted_scanout,
        transfer_route: route,
        fallback_armed: health.fallback_armed(),
    })
}
