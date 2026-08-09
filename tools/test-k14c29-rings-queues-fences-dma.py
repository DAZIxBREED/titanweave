#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
files={p.name:p.read_text() for p in [
 root/'kernel/weavecore/src/radeon_sdma_packets.rs',root/'kernel/weavecore/src/radeon_ring.rs',
 root/'kernel/weavecore/src/radeon_queue.rs',root/'kernel/weavecore/src/radeon_fence.rs',
 root/'kernel/weavecore/src/radeon_dma.rs',root/'kernel/weavecore/src/radeon_sdma.rs',
 root/'kernel/weavecore/src/native_gpu_c29.rs']}
joined='\n'.join(files.values())
for bad in ['todo!()', 'unimplemented!()', 'unimplemented!("', 'todo!("']:
    assert bad not in joined, f'C29 contains forbidden stub primitive: {bad}'
p=files['radeon_sdma_packets.rs']
for token in ['SDMA_OP_COPY:u32=1','SDMA_OP_FENCE:u32=5','SDMA_SUBOP_COPY_LINEAR:u32=0','SDMA_FENCE_MTYPE_UC:u32=3','bytes-1','COPY_LINEAR_DWORDS:usize=8','FENCE_DWORDS:usize=4','decode_copy','decode_fence']:
    assert token in p, token
r=files['radeon_ring.rs']
for token in ['C29_RING_BYTES:u64=16*1024','allocate_gtt','write_volatile','C29_RING_ALIGN_DWORDS:u64=16','ring full','pad_commit_alignment','wraps']:
    assert token in r, token
q=files['radeon_queue.rs']
for token in ['C29_QUEUE_DEPTH:usize=32','SubmissionStatus','Queued=1','Emitted=2','Retired=3','Cancelled=4','retire_head','cancel_all','FIFO retire']:
    assert token in q, token
f=files['radeon_fence.rs']
for token in ['RadeonFenceTimeline','allocate_gtt','write_volatile','read_volatile','is_complete','radeon_sdma_packets::fence']:
    assert token in f, token
d=files['radeon_dma.rs']
for token in ['execute_typed_copy','copy_nonoverlapping','execute_typed_fence','C29_DMA_TEST_BYTES:u32=4096','raw_packets_allowed:false','software qualification']:
    assert token in d, token
s=files['radeon_sdma.rs']
for token in ['SDMA0_QUEUE0_RB_CNTL:u32=0x0080','SDMA0_QUEUE0_RB_BASE:u32=0x0081','SDMA0_QUEUE0_RB_WPTR:u32=0x0085','SDMA_QUEUE_BASE_IDX:u8=0','C29_ARBITRARY_MMIO_ALLOWED:bool=false','native_gpu_c19::with_verified_snapshot','resolve_gfx12_scratch_reg0','hardware_programmed=false']:
    assert token in s, token
n=files['native_gpu_c29.rs']
for token in ['K14C29_ABI_VERSION:u32=1','RADEON_C29_RAW_PACKET_SUBMISSION:bool=false','RADEON_C29_PLACEHOLDER_SUBSYSTEMS:u8=0','radeon_dma::qualification','[C29RG]','[C29QU]','[C29FN]','[C29DM]','[C29SD]','[C29PG]','[C29RD]','persistent_iommu_domain_live:false']:
    assert token in n, token
main=(root/'kernel/weavecore/src/main.rs').read_text();proc=(root/'kernel/weavecore/src/process.rs').read_text();abi=(root/'kernel/weavecore/src/abi.rs').read_text();sys=(root/'kernel/weavecore/src/syscalls.rs').read_text();tw=(root/'userspace/include/twabi.inc').read_text();disp=(root/'userspace/displayd/displayd.S').read_text()
for token in ['mod native_gpu_c29;','mod radeon_ring;','mod radeon_queue;','mod radeon_fence;','mod radeon_dma;','mod radeon_sdma;','[C29OK]']:
    assert token in main, token
assert 'SYS_NATIVE_GPU_C29_QUERY: u64 = 40' in abi
assert 'SYS_NATIVE_GPU_C29_QUERY =>' in sys and 'native_gpu_c29::packed_status' in sys
assert '.equ TW_SYS_NATIVE_GPU_C29_QUERY, 40' in tw
for token in ['K14.C29 Radeon rings, queues, timeline fences, and typed DMA subsystem online','physical SDMA remains safely deferred']:
    assert token in disp, token
for token in ['[KERN] K14.C29 alive:','[QUAL] K14.C29 rings-queues-fences-dma runtime reached intentional post-userspace halt','[HALT] BSP halted intentionally']:
    assert token in proc, token
runner=(root/'tools/run-k14c29-qemu-rings-queues-fences-dma.sh').read_text();checker=(root/'tools/check-k14c29-serial-log.sh').read_text()
for token in ['Intentional Titanweave HALT detected','check-k14c29-serial-log.sh','-device edu,id=twk14c29iommutest']:
    assert token in runner, token
for token in ['[C29RG] SDMA ring:','[C29DM] typed SDMA DMA:','Titanweave K14.C29 rings-queues-fences-dma runtime qualification PASSED.']:
    assert token in checker, token
print('Titanweave K14.C29 operational rings+queues+fences+DMA source checks passed.')
