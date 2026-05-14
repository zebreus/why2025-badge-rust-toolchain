use why2025_badge_sys_bindings as abi;

use crate::ffi::{CStr, c_char, c_int};
use crate::{io, ptr};

pub(crate) type Pid = abi::pid_t;

#[inline]
pub(crate) fn spawn(path: &CStr, argv: &mut [*mut c_char]) -> io::Result<Pid> {
    if argv.len() > c_int::MAX as usize {
        return Err(io::const_error!(
            io::ErrorKind::InvalidInput,
            "too many process arguments for BadgeVMS"
        ));
    }

    let argc = argv.len() as c_int;
    let argv = if argv.is_empty() { ptr::null_mut() } else { argv.as_mut_ptr() };
    let pid = unsafe { abi::process_create(path.as_ptr(), 0, argc, argv) };

    if pid < 0 { Err(io::Error::last_os_error()) } else { Ok(pid) }
}

#[inline]
pub(crate) fn wait(block: bool, timeout_msec: u32) -> io::Result<Option<Pid>> {
    let pid = unsafe { abi::wait(block, timeout_msec) };

    if pid < 0 {
        if block { Err(io::Error::last_os_error()) } else { Ok(None) }
    } else {
        Ok(Some(pid))
    }
}

#[inline]
pub(crate) fn getpid() -> Pid {
    unsafe { abi::getpid() }
}
