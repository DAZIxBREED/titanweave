use crate::sync::SpinLock;

#[derive(Clone, Copy)]
pub struct CachePolicy {
    pub maximum_bytes: u64,
    pub write_back_enabled: bool,
    pub preload_enabled: bool,
}

static POLICY: SpinLock<CachePolicy> = SpinLock::new(CachePolicy {
    maximum_bytes: 32 * 1024 * 1024,
    write_back_enabled: false,
    preload_enabled: true,
});

pub fn initialize(available_memory_bytes: u64) -> CachePolicy {
    let maximum = core::cmp::min(available_memory_bytes / 16, 256 * 1024 * 1024);
    let mut policy = POLICY.lock();
    policy.maximum_bytes = core::cmp::max(maximum, 8 * 1024 * 1024);
    policy.write_back_enabled = false;
    policy.preload_enabled = true;
    *policy
}
