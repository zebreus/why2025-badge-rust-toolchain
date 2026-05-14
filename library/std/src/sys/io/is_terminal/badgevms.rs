use why2025_badge_sys_bindings as abi;

use crate::os::fd::AsRawFd;
use crate::sys::AsInner;

pub trait IsTerminal {
    fn is_terminal(&self) -> bool;
}

impl<T: AsRawFd> IsTerminal for T {
    fn is_terminal(&self) -> bool {
        unsafe { abi::isatty(self.as_raw_fd()) != 0 }
    }
}

impl IsTerminal for crate::fs::File {
    fn is_terminal(&self) -> bool {
        unsafe { abi::isatty(self.as_inner().as_raw_fd()) != 0 }
    }
}

pub fn is_terminal<T: IsTerminal>(fd: &T) -> bool {
    fd.is_terminal()
}
