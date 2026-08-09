//! K13 bounded command-submission queue contract.

pub const GPU_QUEUE_DEPTH: usize = 64;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandKind {
    Nop = 0,
    Blit = 1,
    Clear = 2,
    Present = 3,
    Compute = 4,
    CopyBetweenAdapters = 5,
}

#[derive(Clone, Copy, Debug)]
pub struct CommandPacket {
    pub kind: CommandKind,
    pub flags: u32,
    pub buffer_id: u64,
    pub offset: u64,
    pub bytes: u64,
    pub fence_value: u64,
}

impl CommandPacket {
    pub const EMPTY: Self = Self {
        kind: CommandKind::Nop,
        flags: 0,
        buffer_id: 0,
        offset: 0,
        bytes: 0,
        fence_value: 0,
    };
}

pub struct CommandQueue {
    entries: [CommandPacket; GPU_QUEUE_DEPTH],
    head: usize,
    tail: usize,
    count: usize,
}

impl CommandQueue {
    pub const fn new() -> Self {
        Self { entries: [CommandPacket::EMPTY; GPU_QUEUE_DEPTH], head: 0, tail: 0, count: 0 }
    }

    pub fn submit(&mut self, packet: CommandPacket) -> Result<(), &'static str> {
        if self.count == GPU_QUEUE_DEPTH { return Err("GPU command queue full"); }
        if packet.kind != CommandKind::Nop && packet.fence_value == 0 {
            return Err("GPU command missing fence value");
        }
        self.entries[self.tail] = packet;
        self.tail = (self.tail + 1) % GPU_QUEUE_DEPTH;
        self.count += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Option<CommandPacket> {
        if self.count == 0 { return None; }
        let packet = self.entries[self.head];
        self.entries[self.head] = CommandPacket::EMPTY;
        self.head = (self.head + 1) % GPU_QUEUE_DEPTH;
        self.count -= 1;
        Some(packet)
    }

    #[must_use] pub const fn len(&self) -> usize { self.count }
}

pub fn run_self_test() -> Result<usize, &'static str> {
    let mut queue = CommandQueue::new();
    for index in 0..4u64 {
        queue.submit(CommandPacket {
            kind: if index == 3 { CommandKind::Present } else { CommandKind::Blit },
            flags: 0,
            buffer_id: index + 1,
            offset: 0,
            bytes: 4096,
            fence_value: index + 1,
        })?;
    }
    if queue.len() != 4 { return Err("GPU queue depth self-test failed"); }
    let first = queue.pop().ok_or("GPU queue unexpectedly empty")?;
    if first.fence_value != 1 || queue.len() != 3 { return Err("GPU queue ordering self-test failed"); }
    Ok(4)
}
