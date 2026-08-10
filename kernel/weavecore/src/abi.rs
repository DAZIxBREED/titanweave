pub const SYSCALL_VECTOR: u8 = 0x80;
pub const SYS_EXIT: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_CHANNEL_SEND: u64 = 2;
pub const SYS_CHANNEL_RECEIVE: u64 = 3;
pub const SYS_GETPID: u64 = 4;
pub const SYS_YIELD: u64 = 5;
pub const SYS_SYSTEM_QUERY: u64 = 6;
pub const SYS_DISPLAY_QUERY: u64 = 7;
pub const SYS_GPU_QUERY: u64 = 8;
pub const SYS_GPU_PRESENT: u64 = 9;
pub const SYS_GPU_RECOVER: u64 = 10;
pub const SYS_NATIVE_GPU_QUERY: u64 = 11;
pub const SYS_NATIVE_GPU_BINDING_QUERY: u64 = 12;
pub const SYS_NATIVE_GPU_C2_QUERY: u64 = 13;
pub const SYS_NATIVE_GPU_C3_QUERY: u64 = 14;
pub const SYS_NATIVE_GPU_C4_QUERY: u64 = 15;
pub const SYS_NATIVE_GPU_C5_QUERY: u64 = 16;
pub const SYS_NATIVE_GPU_C6_QUERY: u64 = 17;
pub const SYS_NATIVE_GPU_C7_QUERY: u64 = 18;
pub const SYS_NATIVE_GPU_C8_QUERY: u64 = 19;
pub const SYS_NATIVE_GPU_C9_QUERY: u64 = 20;
pub const SYS_NATIVE_GPU_C10_QUERY: u64 = 21;
pub const SYS_NATIVE_GPU_C11_QUERY: u64 = 22;
pub const SYS_NATIVE_GPU_C12_QUERY: u64 = 23;
pub const SYS_NATIVE_GPU_C13_QUERY: u64 = 24;
pub const SYS_NATIVE_GPU_C14_QUERY: u64 = 25;
pub const SYS_NATIVE_GPU_C15_QUERY: u64 = 26;
pub const SYS_NATIVE_GPU_C16_QUERY: u64 = 27;
pub const SYS_NATIVE_GPU_C17_QUERY: u64 = 28;
pub const SYS_NATIVE_GPU_C18_QUERY: u64 = 29;
pub const SYS_NATIVE_GPU_C19_QUERY: u64 = 30;
pub const SYS_NATIVE_GPU_C20_QUERY: u64 = 31;
pub const SYS_NATIVE_GPU_C21_QUERY: u64 = 32;
pub const SYS_NATIVE_GPU_C22_QUERY: u64 = 33;
pub const SYS_NATIVE_GPU_C23_QUERY: u64 = 34;
pub const SYS_NATIVE_GPU_C24_QUERY: u64 = 35;
pub const SYS_NATIVE_GPU_C25_QUERY: u64 = 36;
pub const SYS_NATIVE_GPU_C26_QUERY: u64 = 37;
pub const SYS_NATIVE_GPU_C27_QUERY: u64 = 38;
pub const SYS_NATIVE_GPU_C28_QUERY: u64 = 39;
pub const SYS_NATIVE_GPU_C29_QUERY: u64 = 40;
pub const SYS_NATIVE_GPU_C30_QUERY: u64 = 41;
pub const SYS_NATIVE_GPU_C31_QUERY: u64 = 42;
pub const SYS_NATIVE_GPU_C32_QUERY: u64 = 43;
pub const SYS_AUDIO_ABI_QUERY: u64 = 44;
pub const SYS_AUDIO_ENUMERATE: u64 = 45;
pub const SYS_AUDIO_CONTROL: u64 = 46;
pub const SYS_AUDIO_SERVER_CONTROL: u64 = 47;
pub const SYS_AUDIO_TRANSPORT_CONTROL: u64 = 48;

pub const ERROR_INVALID_ARGUMENT: i64 = -1;
pub const ERROR_BAD_HANDLE: i64 = -2;
pub const ERROR_ACCESS_DENIED: i64 = -3;
pub const ERROR_WOULD_BLOCK: i64 = -4;
pub const ERROR_NO_SPACE: i64 = -5;
pub const ERROR_PROCESS_FAULT: i64 = -6;
pub const ERROR_NOT_FOUND: i64 = -7;
pub const ERROR_NOT_READY: i64 = -8;
pub const ERROR_BUSY: i64 = -9;
pub const ERROR_BUFFER_TOO_SMALL: i64 = -10;
pub const ERROR_INVALID_STATE: i64 = -11;
pub const ERROR_NOT_SUPPORTED: i64 = -12;

#[inline]
pub const fn encode_error(error: i64) -> u64 {
    error as u64
}
