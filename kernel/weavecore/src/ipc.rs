use crate::handles::HandleDescriptor;
use crate::objects::{ObjectHeader, ObjectKind};
use crate::sync::SpinLock;

pub const MAX_MESSAGE_BYTES: usize = 256;
pub const CHANNEL_QUEUE_DEPTH: usize = 16;
pub const SERVICE_CHANNEL_INDEX: u8 = 0;

#[derive(Clone, Copy)]
pub struct ChannelMessage {
    pub bytes: [u8; MAX_MESSAGE_BYTES],
    pub length: usize,
    pub capability: Option<HandleDescriptor>,
}
impl ChannelMessage {
    const EMPTY: Self = Self { bytes: [0; MAX_MESSAGE_BYTES], length: 0, capability: None };
}

#[derive(Clone, Copy)]
struct MessageQueue {
    messages: [ChannelMessage; CHANNEL_QUEUE_DEPTH],
    head: usize,
    tail: usize,
    len: usize,
    closed: bool,
}
impl MessageQueue {
    const fn new() -> Self {
        Self { messages: [ChannelMessage::EMPTY; CHANNEL_QUEUE_DEPTH], head: 0, tail: 0, len: 0, closed: false }
    }
    fn push(&mut self, message: ChannelMessage) -> Result<(), &'static str> {
        if self.closed { return Err("IPC peer endpoint is closed"); }
        if self.len == CHANNEL_QUEUE_DEPTH { return Err("IPC queue is full"); }
        self.messages[self.tail] = message;
        self.tail = (self.tail + 1) % CHANNEL_QUEUE_DEPTH;
        self.len += 1;
        Ok(())
    }
    fn pop(&mut self) -> Result<ChannelMessage, &'static str> {
        if self.len == 0 {
            return if self.closed { Err("IPC endpoint is closed") } else { Err("IPC queue is empty") };
        }
        let message = self.messages[self.head];
        self.messages[self.head] = ChannelMessage::EMPTY;
        self.head = (self.head + 1) % CHANNEL_QUEUE_DEPTH;
        self.len -= 1;
        Ok(message)
    }
}

struct ChannelState { queues: [MessageQueue; 2], endpoint_open: [bool; 2] }
impl ChannelState {
    const fn new() -> Self { Self { queues: [MessageQueue::new(), MessageQueue::new()], endpoint_open: [true, true] } }
}

pub struct ChannelPair { header: ObjectHeader, state: SpinLock<ChannelState> }
impl ChannelPair {
    pub const fn new_static(id: u64) -> Self {
        Self { header: ObjectHeader::new_static(id, ObjectKind::Channel), state: SpinLock::new(ChannelState::new()) }
    }
    #[must_use] pub const fn header(&self) -> &ObjectHeader { &self.header }
    pub fn send(&self, sender_side: u8, bytes: &[u8], capability: Option<HandleDescriptor>) -> Result<(), &'static str> {
        if sender_side > 1 || bytes.is_empty() || bytes.len() > MAX_MESSAGE_BYTES { return Err("IPC send arguments are invalid"); }
        let peer = (sender_side ^ 1) as usize;
        let mut state = self.state.lock();
        if !state.endpoint_open[sender_side as usize] { return Err("sending IPC endpoint is closed"); }
        if !state.endpoint_open[peer] { return Err("peer IPC endpoint is closed"); }
        let mut message = ChannelMessage::EMPTY;
        message.bytes[..bytes.len()].copy_from_slice(bytes);
        message.length = bytes.len();
        message.capability = capability;
        state.queues[peer].push(message)
    }
    pub fn receive(&self, receiver_side: u8) -> Result<ChannelMessage, &'static str> {
        if receiver_side > 1 { return Err("IPC endpoint side is invalid"); }
        self.state.lock().queues[receiver_side as usize].pop()
    }
    pub fn close(&self, side: u8) -> Result<(), &'static str> {
        if side > 1 { return Err("IPC endpoint side is invalid"); }
        let mut state = self.state.lock();
        state.endpoint_open[side as usize] = false;
        state.queues[(side ^ 1) as usize].closed = true;
        Ok(())
    }
    pub fn queued(&self, side: u8) -> Result<usize, &'static str> {
        if side > 1 { return Err("IPC endpoint side is invalid"); }
        Ok(self.state.lock().queues[side as usize].len)
    }
}

pub static SERVICE_CHANNEL: ChannelPair = ChannelPair::new_static(0x3000);
