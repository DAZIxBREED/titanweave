use crate::sync::SpinLock;

#[derive(Clone, Copy)]
pub struct BootHealth {
    pub failed_boots: u32,
    pub recovery_required: bool,
    pub cache_reset_required: bool,
}

static HEALTH: SpinLock<BootHealth> = SpinLock::new(BootHealth {
    failed_boots: 0,
    recovery_required: false,
    cache_reset_required: false,
});

pub fn initialize() -> BootHealth {
    *HEALTH.lock()
}

/// Called only after kernel initialization and native-service loading have
/// completed successfully. A panic path must never call this function.
pub fn mark_boot_stable() {
    let mut health = HEALTH.lock();
    health.failed_boots = 0;
    health.recovery_required = false;
    health.cache_reset_required = false;
}

/// Records an abnormal kernel termination. Saturating accounting avoids a
/// second failure turning the recovery counter into a false success by wrap.
pub fn mark_boot_failed() {
    let mut health = HEALTH.lock();
    health.failed_boots = health.failed_boots.saturating_add(1);
    health.recovery_required = true;
    health.cache_reset_required = true;
}
