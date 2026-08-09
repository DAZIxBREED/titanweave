//! K12/K13 display/input routing policy.
//!
//! Hardware HID decoding remains in the K11 USB stack. K12/K13 turns decoded input
//! into compositor-facing focus/capture events without giving applications raw
//! device ownership.

use crate::graphics_abi::{InputEvent, InputEventKind};

pub const MAX_INPUT_EVENTS: usize = 128;

pub struct InputRouter {
    events: [InputEvent; MAX_INPUT_EVENTS],
    head: usize,
    tail: usize,
    count: usize,
    sequence: u64,
    pointer_x: i32,
    pointer_y: i32,
    display_width: u32,
    display_height: u32,
    keyboard_focus: u64,
    pointer_focus: u64,
    pointer_capture: u64,
}

impl InputRouter {
    pub const fn new(display_width: u32, display_height: u32) -> Self {
        Self {
            events: [InputEvent {
                sequence: 0,
                timestamp_ticks: 0,
                target_surface: 0,
                kind: 0,
                code: 0,
                value_x: 0,
                value_y: 0,
            }; MAX_INPUT_EVENTS],
            head: 0,
            tail: 0,
            count: 0,
            sequence: 0,
            pointer_x: 0,
            pointer_y: 0,
            display_width,
            display_height,
            keyboard_focus: 0,
            pointer_focus: 0,
            pointer_capture: 0,
        }
    }

    pub fn set_keyboard_focus(&mut self, surface: u64) {
        self.keyboard_focus = surface;
    }

    pub fn set_pointer_focus(&mut self, surface: u64) {
        if self.pointer_capture == 0 {
            self.pointer_focus = surface;
        }
    }

    pub fn capture_pointer(&mut self, surface: u64) -> Result<(), &'static str> {
        if surface == 0 {
            return Err("pointer capture requires a surface");
        }
        if self.pointer_capture != 0 && self.pointer_capture != surface {
            return Err("pointer is already captured");
        }
        self.pointer_capture = surface;
        Ok(())
    }

    pub fn release_pointer(&mut self, surface: u64) {
        if self.pointer_capture == surface {
            self.pointer_capture = 0;
        }
    }

    pub fn route_key(&mut self, pressed: bool, key_code: u32, ticks: u64) -> Result<(), &'static str> {
        let target = self.keyboard_focus;
        let sequence = self.next_sequence();
        self.push(InputEvent {
            sequence,
            timestamp_ticks: ticks,
            target_surface: target,
            kind: if pressed { InputEventKind::KeyDown as u32 } else { InputEventKind::KeyUp as u32 },
            code: key_code,
            value_x: 0,
            value_y: 0,
        })
    }

    pub fn route_pointer_move(&mut self, delta_x: i32, delta_y: i32, ticks: u64) -> Result<(), &'static str> {
        let max_x = self.display_width.saturating_sub(1).min(i32::MAX as u32) as i32;
        let max_y = self.display_height.saturating_sub(1).min(i32::MAX as u32) as i32;
        self.pointer_x = self.pointer_x.saturating_add(delta_x).clamp(0, max_x);
        self.pointer_y = self.pointer_y.saturating_add(delta_y).clamp(0, max_y);
        let target = if self.pointer_capture != 0 { self.pointer_capture } else { self.pointer_focus };
        let sequence = self.next_sequence();
        let pointer_x = self.pointer_x;
        let pointer_y = self.pointer_y;
        self.push(InputEvent {
            sequence,
            timestamp_ticks: ticks,
            target_surface: target,
            kind: InputEventKind::PointerMove as u32,
            code: 0,
            value_x: pointer_x,
            value_y: pointer_y,
        })
    }

    pub fn route_button(&mut self, pressed: bool, button: u32, ticks: u64) -> Result<(), &'static str> {
        let target = if self.pointer_capture != 0 { self.pointer_capture } else { self.pointer_focus };
        let sequence = self.next_sequence();
        let pointer_x = self.pointer_x;
        let pointer_y = self.pointer_y;
        self.push(InputEvent {
            sequence,
            timestamp_ticks: ticks,
            target_surface: target,
            kind: if pressed {
                InputEventKind::PointerButtonDown as u32
            } else {
                InputEventKind::PointerButtonUp as u32
            },
            code: button,
            value_x: pointer_x,
            value_y: pointer_y,
        })
    }

    pub fn pop(&mut self) -> Option<InputEvent> {
        if self.count == 0 {
            return None;
        }
        let event = self.events[self.head];
        self.head = (self.head + 1) % MAX_INPUT_EVENTS;
        self.count -= 1;
        Some(event)
    }

    #[must_use]
    pub const fn pending(&self) -> usize {
        self.count
    }

    fn next_sequence(&mut self) -> u64 {
        self.sequence = self.sequence.saturating_add(1);
        self.sequence
    }

    fn push(&mut self, event: InputEvent) -> Result<(), &'static str> {
        if self.count == MAX_INPUT_EVENTS {
            return Err("input event queue is full");
        }
        self.events[self.tail] = event;
        self.tail = (self.tail + 1) % MAX_INPUT_EVENTS;
        self.count += 1;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct InputSelfTestReport {
    pub events: usize,
    pub final_target: u64,
    pub pointer_x: i32,
    pub pointer_y: i32,
}

pub fn run_self_test(width: u32, height: u32) -> Result<InputSelfTestReport, &'static str> {
    let mut router = InputRouter::new(width, height);
    router.set_keyboard_focus(1);
    router.set_pointer_focus(2);
    router.route_key(true, 0x04, 1)?;
    router.route_pointer_move(12, 9, 2)?;
    router.capture_pointer(2)?;
    router.route_button(true, 1, 3)?;
    router.route_pointer_move(-4, 3, 4)?;
    router.release_pointer(2);

    let mut events = 0usize;
    let mut final_target = 0u64;
    let mut pointer_x = 0i32;
    let mut pointer_y = 0i32;
    while let Some(event) = router.pop() {
        if event.sequence == 0 {
            return Err("input sequence was not assigned");
        }
        events += 1;
        final_target = event.target_surface;
        pointer_x = event.value_x;
        pointer_y = event.value_y;
    }
    if events != 4 || final_target != 2 || pointer_x != 8 || pointer_y != 12 {
        return Err("input routing self-test produced unexpected state");
    }
    Ok(InputSelfTestReport { events, final_target, pointer_x, pointer_y })
}
