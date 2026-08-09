use crate::arch::x86_64::port::{inb, outb};
use crate::sync::SpinLock;
use core::fmt::{self, Write};

static SERIAL_LOCK: SpinLock<()> = SpinLock::new(());

pub fn initialize() {
    let _guard = SERIAL_LOCK.lock();
    unsafe {
        outb(SerialPort::BASE + 1, 0x00);
        outb(SerialPort::BASE + 3, 0x80);
        outb(SerialPort::BASE, 0x03);
        outb(SerialPort::BASE + 1, 0x00);
        outb(SerialPort::BASE + 3, 0x03);
        outb(SerialPort::BASE + 2, 0xc7);
        outb(SerialPort::BASE + 4, 0x0b);
    }
}

pub fn println(args: fmt::Arguments<'_>) {
    let _guard = SERIAL_LOCK.lock();
    let mut serial = SerialPort;
    let _ = serial.write_fmt(args);
    let _ = serial.write_str("\n");
}

/// Emit a bounded user-mode payload as one diagnostic line. K6 keeps console
/// output behind a process handle rather than exposing the UART to ring 3.
pub fn write_user_bytes(bytes: &[u8]) {
    let _guard = SERIAL_LOCK.lock();
    let mut serial = SerialPort;
    let _ = serial.write_str("[USER] ");
    for &byte in bytes {
        let printable = if byte.is_ascii_graphic() || byte == b' ' { byte } else { b'?' };
        SerialPort::write_byte(printable);
    }
    let _ = serial.write_str("\n");
}

/// Best-effort output for panic, NMI, and double-fault paths. This deliberately
/// bypasses the normal lock because the interrupted context may already own it.
pub fn emergency_println(args: fmt::Arguments<'_>) {
    let mut serial = SerialPort;
    let _ = serial.write_fmt(args);
    let _ = serial.write_str("\n");
}

struct SerialPort;

impl SerialPort {
    const BASE: u16 = 0x3f8;

    fn write_byte(byte: u8) {
        unsafe {
            while inb(Self::BASE + 5) & 0x20 == 0 {
                core::hint::spin_loop();
            }
            outb(Self::BASE, byte);
        }
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for byte in text.bytes() {
            if byte == b'\n' {
                Self::write_byte(b'\r');
            }
            Self::write_byte(byte);
        }
        Ok(())
    }
}
