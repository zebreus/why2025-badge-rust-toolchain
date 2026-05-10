use why2025_badge_sys_bindings as abi;

use crate::ffi::c_int;
use crate::io;

#[inline]
pub fn errno_location() -> *mut c_int {
    unsafe { abi::__errno() }
}

#[inline]
pub fn errno() -> i32 {
    unsafe { *errno_location() as i32 }
}

#[inline]
#[allow(dead_code)]
pub fn set_errno(e: i32) {
    unsafe { *errno_location() = e as c_int }
}

#[inline]
pub fn is_interrupted(errno: i32) -> bool {
    errno == 4
}

pub fn decode_error_kind(errno: i32) -> io::ErrorKind {
    use io::ErrorKind::*;

    match errno {
        1 | 13 => PermissionDenied,
        2 => NotFound,
        4 => Interrupted,
        11 => WouldBlock,
        12 => OutOfMemory,
        17 => AlreadyExists,
        20 => NotADirectory,
        21 => IsADirectory,
        22 => InvalidInput,
        28 => StorageFull,
        32 => BrokenPipe,
        38 | 95 => Unsupported,
        _ => Uncategorized,
    }
}

pub fn error_string(errno: i32) -> String {
    format!("BadgeVMS OS error {errno}")
}
