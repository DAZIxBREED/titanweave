//! Kernel-facing contract for the user-space Titan Archive Service (TAS).

use crate::{archive, package, serial};

pub const ARCHIVE_PROTOCOL_VERSION: u32 = 2;
pub const MAX_ARCHIVE_JOBS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveOperation { Probe, List, Test, Extract, Create, InstallPackage }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveJobState { Empty, Queued, Running, Completed, Failed, Cancelled }

#[derive(Clone, Copy, Debug)]
pub struct ArchiveJob {
    pub id: u64,
    pub operation: ArchiveOperation,
    pub state: ArchiveJobState,
    pub format: archive::ArchiveFormat,
    pub source_volume: [u8;16],
    pub destination_volume: [u8;16],
    pub input_bytes: u64,
    pub output_limit: u64,
    pub worker_pid: u64,
    pub result_code: i32,
}
impl ArchiveJob { pub const EMPTY: Self = Self { id:0, operation:ArchiveOperation::Probe,
    state:ArchiveJobState::Empty, format:archive::ArchiveFormat::Unknown,
    source_volume:[0;16], destination_volume:[0;16], input_bytes:0, output_limit:0,
    worker_pid:0, result_code:0 }; }

pub struct ArchiveQueue { jobs:[ArchiveJob;MAX_ARCHIVE_JOBS], next_id:u64 }
impl ArchiveQueue {
    pub const fn new()->Self{Self{jobs:[ArchiveJob::EMPTY;MAX_ARCHIVE_JOBS],next_id:1}}

    pub fn submit(&mut self, mut job:ArchiveJob)->Result<u64,&'static str>{
        if job.output_limit == 0 && matches!(job.operation, ArchiveOperation::Extract | ArchiveOperation::InstallPackage) {
            return Err("extract/install job requires an output limit");
        }
        for slot in &mut self.jobs { if slot.state==ArchiveJobState::Empty {
            job.id=self.next_id; self.next_id=self.next_id.wrapping_add(1).max(1);
            job.state=ArchiveJobState::Queued; job.worker_pid=0; job.result_code=0; *slot=job; return Ok(job.id)
        }} Err("archive job queue full")
    }

    pub fn claim_next(&mut self, worker_pid:u64)->Result<ArchiveJob,&'static str>{
        if worker_pid==0{return Err("invalid archive worker pid")}
        for slot in &mut self.jobs { if slot.state==ArchiveJobState::Queued {
            slot.state=ArchiveJobState::Running; slot.worker_pid=worker_pid; return Ok(*slot)
        }} Err("no queued archive job")
    }

    fn worker_slot(&mut self,id:u64,worker_pid:u64)->Result<&mut ArchiveJob,&'static str>{
        for slot in &mut self.jobs { if slot.id==id && slot.state!=ArchiveJobState::Empty {
            if slot.state!=ArchiveJobState::Running{return Err("archive job is not running")}
            if slot.worker_pid!=worker_pid{return Err("archive worker does not own job")}
            return Ok(slot)
        }} Err("archive job not found")
    }

    pub fn complete(&mut self,id:u64,worker_pid:u64)->Result<(),&'static str>{
        let slot=self.worker_slot(id,worker_pid)?; slot.state=ArchiveJobState::Completed; slot.result_code=0; Ok(())
    }

    pub fn fail(&mut self,id:u64,worker_pid:u64,result_code:i32)->Result<(),&'static str>{
        if result_code==0{return Err("failed archive job requires nonzero result")}
        let slot=self.worker_slot(id,worker_pid)?; slot.state=ArchiveJobState::Failed; slot.result_code=result_code; Ok(())
    }

    pub fn cancel(&mut self,id:u64)->Result<(),&'static str>{
        for slot in &mut self.jobs{if slot.id==id && slot.state!=ArchiveJobState::Empty{
            match slot.state { ArchiveJobState::Completed|ArchiveJobState::Failed=>return Err("finished archive job cannot be cancelled"), _=>{slot.state=ArchiveJobState::Cancelled;return Ok(())} }
        }}Err("archive job not found")
    }

    pub fn reap(&mut self,id:u64)->Result<ArchiveJob,&'static str>{
        for slot in &mut self.jobs{if slot.id==id{
            match slot.state { ArchiveJobState::Completed|ArchiveJobState::Failed|ArchiveJobState::Cancelled=>{let finished=*slot;*slot=ArchiveJob::EMPTY;return Ok(finished)}, _=>return Err("archive job has not finished") }
        }}Err("archive job not found")
    }
}

pub fn initialize() {
    let sample = [0x37,0x7a,0xbc,0xaf,0x27,0x1c,0,4,0,0,0,0];
    let probe = archive::probe(&sample);
    let identity = package::PackageIdentity { id_hash: package::fnv1a64(b"titanweave.k9.selftest"), version_major:0, version_minor:9, version_patch:1, kind:package::PackageKind::Generic };
    let mut tx = package::PackageTransaction::new(identity);
    let transaction_ok = tx.mark_verified(true,true).and_then(|_|tx.stage(1,4096)).and_then(|_|tx.prepare_rollback()).and_then(|_|tx.commit()).is_ok() && tx.journal_record().validate().is_ok();
    serial::println(format_args!("[ARCH] Vaultforge protocol={} primary={} capability={:?} journal_selftest={}", ARCHIVE_PROTOCOL_VERSION, probe.format.name(), probe.capability, transaction_ok));
}
