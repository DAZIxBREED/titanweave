//! K13 timeline-fence model shared by graphics backends.

#[derive(Clone, Copy, Debug)]
pub struct TimelineFence {
    submitted: u64,
    completed: u64,
}

impl TimelineFence {
    pub const fn new() -> Self { Self { submitted: 0, completed: 0 } }

    pub fn issue(&mut self) -> Result<u64, &'static str> {
        self.submitted = self.submitted.checked_add(1).ok_or("GPU timeline exhausted")?;
        Ok(self.submitted)
    }

    pub fn complete(&mut self, value: u64) -> Result<(), &'static str> {
        if value < self.completed || value > self.submitted {
            return Err("invalid GPU fence completion");
        }
        self.completed = value;
        Ok(())
    }

    #[must_use] pub const fn is_complete(&self, value: u64) -> bool { value <= self.completed }
    #[must_use] pub const fn submitted(&self) -> u64 { self.submitted }
    #[must_use] pub const fn completed(&self) -> u64 { self.completed }
}

pub fn run_self_test() -> Result<(u64, u64), &'static str> {
    let mut timeline = TimelineFence::new();
    let first = timeline.issue()?;
    let second = timeline.issue()?;
    if timeline.is_complete(second) { return Err("GPU fence completed before signal"); }
    timeline.complete(first)?;
    if !timeline.is_complete(first) || timeline.is_complete(second) {
        return Err("GPU fence ordering self-test failed");
    }
    timeline.complete(second)?;
    Ok((timeline.submitted(), timeline.completed()))
}
