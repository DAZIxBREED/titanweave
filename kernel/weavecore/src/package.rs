//! K9 transactional Titanweave package metadata and restartable journal records.

pub const MANIFEST_PATH: &[u8] = b"titanweave/manifest.toml";
pub const SIGNATURE_PATH: &[u8] = b"titanweave/signature";
pub const CHECKSUMS_PATH: &[u8] = b"titanweave/checksums";
pub const PACKAGE_JOURNAL_MAGIC: u64 = 0x5457_504b_474a_4e4c; // "TWPKGJNL"
pub const PACKAGE_JOURNAL_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageKind { Application, Driver, Update, Theme, Firmware, Backup, Generic }

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransactionState {
    Created = 0, Verified = 1, Staged = 2, RollbackReady = 3,
    Committing = 4, Committed = 5, RollingBack = 6, RolledBack = 7, Failed = 8,
}

#[derive(Clone, Copy, Debug)]
pub struct PackageIdentity {
    pub id_hash: u64,
    pub version_major: u16,
    pub version_minor: u16,
    pub version_patch: u16,
    pub kind: PackageKind,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PackageJournalRecord {
    pub magic: u64,
    pub version: u16,
    pub state: u8,
    pub reserved: u8,
    pub sequence: u64,
    pub package_id_hash: u64,
    pub staged_files: u32,
    pub staged_bytes: u64,
    pub applied_files: u32,
    pub checksum: u64,
}

impl PackageJournalRecord {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.magic != PACKAGE_JOURNAL_MAGIC { return Err("invalid package journal magic"); }
        if self.version != PACKAGE_JOURNAL_VERSION { return Err("unsupported package journal version"); }
        if self.state > TransactionState::Failed as u8 { return Err("invalid package journal state"); }
        if self.checksum != self.calculate_checksum() { return Err("package journal checksum mismatch"); }
        if self.applied_files > self.staged_files { return Err("package journal applied count exceeds staged count"); }
        Ok(())
    }

    pub fn calculate_checksum(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325u64;
        for value in [
            self.magic, self.version as u64, self.state as u64, self.sequence,
            self.package_id_hash, self.staged_files as u64, self.staged_bytes,
            self.applied_files as u64,
        ] {
            for byte in value.to_le_bytes() { hash ^= byte as u64; hash = hash.wrapping_mul(0x100000001b3); }
        }
        hash
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PackageTransaction {
    pub identity: PackageIdentity,
    pub state: TransactionState,
    pub sequence: u64,
    pub staged_files: u32,
    pub staged_bytes: u64,
    pub applied_files: u32,
    pub signature_verified: bool,
    pub checksums_verified: bool,
}

impl PackageTransaction {
    pub const fn new(identity: PackageIdentity) -> Self {
        Self { identity, state: TransactionState::Created, sequence: 1,
            staged_files: 0, staged_bytes: 0, applied_files: 0,
            signature_verified: false, checksums_verified: false }
    }

    fn advance(&mut self, state: TransactionState) { self.sequence = self.sequence.wrapping_add(1).max(1); self.state = state; }

    pub fn mark_verified(&mut self, signature: bool, checksums: bool) -> Result<(), &'static str> {
        if self.state != TransactionState::Created { return Err("verification is only valid for a new transaction"); }
        if !signature || !checksums { self.advance(TransactionState::Failed); return Err("package verification failed"); }
        self.signature_verified = true; self.checksums_verified = true; self.advance(TransactionState::Verified); Ok(())
    }

    pub fn stage(&mut self, files: u32, bytes: u64) -> Result<(), &'static str> {
        if self.state != TransactionState::Verified { return Err("package must be verified before staging"); }
        if files == 0 { return Err("package stage contains no files"); }
        self.staged_files = files; self.staged_bytes = bytes; self.applied_files = 0;
        self.advance(TransactionState::Staged); Ok(())
    }

    pub fn prepare_rollback(&mut self) -> Result<(), &'static str> {
        if self.state != TransactionState::Staged { return Err("package must be staged before rollback preparation"); }
        self.advance(TransactionState::RollbackReady); Ok(())
    }

    pub fn begin_commit(&mut self) -> Result<(), &'static str> {
        if self.state != TransactionState::RollbackReady { return Err("rollback point required before commit"); }
        self.advance(TransactionState::Committing); Ok(())
    }

    pub fn record_applied_file(&mut self) -> Result<(), &'static str> {
        if self.state != TransactionState::Committing { return Err("package is not committing"); }
        if self.applied_files >= self.staged_files { return Err("all staged files are already applied"); }
        self.applied_files += 1; self.sequence = self.sequence.wrapping_add(1).max(1); Ok(())
    }

    pub fn finish_commit(&mut self) -> Result<(), &'static str> {
        if self.state != TransactionState::Committing { return Err("package is not committing"); }
        if self.applied_files != self.staged_files { return Err("cannot commit before every staged file is applied"); }
        self.advance(TransactionState::Committed); Ok(())
    }

    pub fn commit(&mut self) -> Result<(), &'static str> {
        self.begin_commit()?;
        while self.applied_files < self.staged_files { self.record_applied_file()?; }
        self.finish_commit()
    }

    pub fn begin_rollback(&mut self) -> Result<(), &'static str> {
        match self.state {
            TransactionState::RollbackReady | TransactionState::Committing | TransactionState::Failed => {
                self.advance(TransactionState::RollingBack); Ok(())
            }
            _ => Err("transaction has no rollback point"),
        }
    }

    pub fn finish_rollback(&mut self) -> Result<(), &'static str> {
        if self.state != TransactionState::RollingBack { return Err("package is not rolling back"); }
        self.applied_files = 0; self.advance(TransactionState::RolledBack); Ok(())
    }

    pub fn rollback(&mut self) { if self.begin_rollback().is_ok() { let _ = self.finish_rollback(); } }

    pub fn journal_record(&self) -> PackageJournalRecord {
        let mut record = PackageJournalRecord {
            magic: PACKAGE_JOURNAL_MAGIC, version: PACKAGE_JOURNAL_VERSION,
            state: self.state as u8, reserved: 0, sequence: self.sequence,
            package_id_hash: self.identity.id_hash, staged_files: self.staged_files,
            staged_bytes: self.staged_bytes, applied_files: self.applied_files, checksum: 0,
        };
        record.checksum = record.calculate_checksum(); record
    }
}

pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes { hash ^= *byte as u64; hash = hash.wrapping_mul(0x100000001b3); }
    hash
}
