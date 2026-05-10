use why2025_badge_sys_bindings as abi;

use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut};

const STDIN_FILENO: i32 = 0;
const STDOUT_FILENO: i32 = 1;
const STDERR_FILENO: i32 = 2;
const EBADF: i32 = 9;

pub struct Stdin;
pub struct Stdout;
pub struct Stderr;

impl Stdin {
    pub const fn new() -> Stdin {
        Stdin
    }
}

impl io::Read for Stdin {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        cvt(unsafe { abi::read(STDIN_FILENO, buf.as_mut_ptr().cast(), buf.len()) })
    }

    fn read_buf(&mut self, mut cursor: BorrowedCursor<'_>) -> io::Result<()> {
        let n = cvt(unsafe {
            abi::read(STDIN_FILENO, cursor.as_mut().as_mut_ptr().cast(), cursor.capacity())
        })?;
        unsafe { cursor.advance(n) };
        Ok(())
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        io::default_read_vectored(|buf| self.read(buf), bufs)
    }

    #[inline]
    fn is_read_vectored(&self) -> bool {
        false
    }
}

impl Stdout {
    pub const fn new() -> Stdout {
        Stdout
    }
}

impl io::Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_fd(STDOUT_FILENO, buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        io::default_write_vectored(|buf| self.write(buf), bufs)
    }

    #[inline]
    fn is_write_vectored(&self) -> bool {
        false
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Stderr {
    pub const fn new() -> Stderr {
        Stderr
    }
}

impl io::Write for Stderr {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_fd(STDERR_FILENO, buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        io::default_write_vectored(|buf| self.write(buf), bufs)
    }

    #[inline]
    fn is_write_vectored(&self) -> bool {
        false
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn write_fd(fd: i32, buf: &[u8]) -> io::Result<usize> {
    cvt(unsafe { abi::write(fd, buf.as_ptr().cast(), buf.len()) })
}

fn cvt(ret: isize) -> io::Result<usize> {
    if ret < 0 { Err(io::Error::last_os_error()) } else { Ok(ret as usize) }
}

pub fn is_ebadf(err: &io::Error) -> bool {
    err.raw_os_error() == Some(EBADF)
}

pub const STDIN_BUF_SIZE: usize = 512;

pub fn panic_output() -> Option<impl io::Write> {
    Some(Stderr::new())
}
