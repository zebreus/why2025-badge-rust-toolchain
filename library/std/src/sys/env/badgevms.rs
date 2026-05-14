use core::slice::memchr;

use why2025_badge_sys_bindings as abi;

pub use super::common::Env;
use crate::ffi::{CStr, OsStr, OsString};
use crate::io;
use crate::sys::helpers::run_with_cstr;

pub fn env() -> Env {
    let mut result = Vec::new();
    let mut environ = unsafe { abi::environ };

    if !environ.is_null() {
        loop {
            let entry = unsafe { *environ };
            if entry.is_null() {
                break;
            }

            if let Some(key_value) = parse(unsafe { CStr::from_ptr(entry) }.to_bytes()) {
                result.push(key_value);
            }
            environ = unsafe { environ.add(1) };
        }
    }

    Env::new(result)
}

fn parse(input: &[u8]) -> Option<(OsString, OsString)> {
    if input.is_empty() {
        return None;
    }

    let pos = memchr::memchr(b'=', &input[1..]).map(|p| p + 1)?;
    Some((unsafe { OsString::from_encoded_bytes_unchecked(input[..pos].to_vec()) }, unsafe {
        OsString::from_encoded_bytes_unchecked(input[pos + 1..].to_vec())
    }))
}

pub fn getenv(k: &OsStr) -> Option<OsString> {
    run_with_cstr(k.as_encoded_bytes(), &|k| {
        let value = unsafe { abi::getenv(k.as_ptr()) };
        if value.is_null() {
            Ok(None)
        } else {
            let bytes = unsafe { CStr::from_ptr(value) }.to_bytes().to_vec();
            Ok(Some(unsafe { OsString::from_encoded_bytes_unchecked(bytes) }))
        }
    })
    .ok()
    .flatten()
}

pub unsafe fn setenv(_: &OsStr, _: &OsStr) -> io::Result<()> {
    Err(io::Error::UNSUPPORTED_PLATFORM)
}

pub unsafe fn unsetenv(_: &OsStr) -> io::Result<()> {
    Err(io::Error::UNSUPPORTED_PLATFORM)
}
