//! K13 multi-GPU transfer-policy groundwork.

use crate::forgegraphics::{CAP_MULTI_GPU_COPY, CAP_SHARED_MEMORY};

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferRoute {
    SameAdapter = 1,
    PeerToPeer = 2,
    SharedSystemMemory = 3,
    CpuStaging = 4,
}

#[must_use]
pub const fn choose_route(source_adapter: u64, destination_adapter: u64, source_caps: u64, destination_caps: u64) -> TransferRoute {
    if source_adapter == destination_adapter {
        TransferRoute::SameAdapter
    } else if source_caps & CAP_MULTI_GPU_COPY != 0 && destination_caps & CAP_MULTI_GPU_COPY != 0 {
        TransferRoute::PeerToPeer
    } else if source_caps & CAP_SHARED_MEMORY != 0 && destination_caps & CAP_SHARED_MEMORY != 0 {
        TransferRoute::SharedSystemMemory
    } else {
        TransferRoute::CpuStaging
    }
}

pub fn run_self_test() -> Result<TransferRoute, &'static str> {
    let route = choose_route(1, 2, CAP_MULTI_GPU_COPY | CAP_SHARED_MEMORY, CAP_MULTI_GPU_COPY);
    if route != TransferRoute::PeerToPeer { return Err("multi-GPU route self-test failed"); }
    Ok(route)
}
