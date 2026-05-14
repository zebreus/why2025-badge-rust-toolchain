use why2025_badge_sys_bindings as abi;

use crate::ffi::{CStr, c_int, c_void};
use crate::io;

pub(crate) const O_RDONLY: c_int = 0;
pub(crate) const O_WRONLY: c_int = 1;
pub(crate) const O_RDWR: c_int = 2;
pub(crate) const O_APPEND: c_int = 0o2000;
pub(crate) const O_CREAT: c_int = 0o100;
pub(crate) const O_EXCL: c_int = 0o200;
pub(crate) const O_TRUNC: c_int = 0o1000;

pub(crate) const SEEK_SET: c_int = 0;
pub(crate) const SEEK_CUR: c_int = 1;
pub(crate) const SEEK_END: c_int = 2;

pub(crate) const S_IFMT: abi::mode_t = 0o170000;
pub(crate) const S_IFDIR: abi::mode_t = 0o040000;
pub(crate) const S_IFREG: abi::mode_t = 0o100000;
pub(crate) const S_IFLNK: abi::mode_t = 0o120000;

pub(crate) const DT_UNKNOWN: u8 = 0;
pub(crate) const DT_DIR: u8 = 4;
pub(crate) const DT_REG: u8 = 8;
pub(crate) const DT_LNK: u8 = 10;

pub(crate) const DEFAULT_FILE_MODE: abi::mode_t = 0o666;
pub(crate) const DEFAULT_DIR_MODE: abi::mode_t = 0o777;

#[inline]
pub(crate) fn open(path: &CStr, flags: c_int, mode: abi::mode_t) -> io::Result<c_int> {
    cvt(unsafe { abi::open(path.as_ptr(), flags, mode) })
}

#[inline]
pub(crate) fn read(fd: c_int, buf: &mut [u8]) -> io::Result<usize> {
    cvt_isize(unsafe { abi::read(fd, buf.as_mut_ptr().cast::<c_void>(), buf.len()) })
        .map(|ret| ret as usize)
}

#[inline]
pub(crate) fn write(fd: c_int, buf: &[u8]) -> io::Result<usize> {
    cvt_isize(unsafe { abi::write(fd, buf.as_ptr().cast::<c_void>(), buf.len()) })
        .map(|ret| ret as usize)
}

#[inline]
pub(crate) fn lseek(fd: c_int, offset: abi::off_t, whence: c_int) -> io::Result<u64> {
    let position = unsafe { abi::lseek(fd, offset, whence) };
    if position < 0 { Err(io::Error::last_os_error()) } else { Ok(position as u64) }
}

#[inline]
pub(crate) fn stat(path: &CStr) -> io::Result<abi::stat> {
    let mut stat = crate::mem::MaybeUninit::uninit();
    cvt(unsafe { abi::stat(path.as_ptr(), stat.as_mut_ptr()) })?;
    Ok(unsafe { stat.assume_init() })
}

#[inline]
pub(crate) fn fstat(fd: c_int) -> io::Result<abi::stat> {
    let mut stat = crate::mem::MaybeUninit::uninit();
    cvt(unsafe { abi::fstat(fd, stat.as_mut_ptr()) })?;
    Ok(unsafe { stat.assume_init() })
}

#[inline]
pub(crate) fn mkdir(path: &CStr, mode: abi::mode_t) -> io::Result<()> {
    cvt(unsafe { abi::mkdir(path.as_ptr(), mode) }).map(drop)
}

#[inline]
pub(crate) fn rmdir(path: &CStr) -> io::Result<()> {
    cvt(unsafe { abi::rmdir(path.as_ptr()) }).map(drop)
}

#[inline]
pub(crate) fn unlink(path: &CStr) -> io::Result<()> {
    cvt(unsafe { abi::unlink(path.as_ptr()) }).map(drop)
}

#[inline]
pub(crate) fn rename(old: &CStr, new: &CStr) -> io::Result<()> {
    cvt(unsafe { abi::rename(old.as_ptr(), new.as_ptr()) }).map(drop)
}

#[inline]
pub(crate) fn link(old: &CStr, new: &CStr) -> io::Result<()> {
    cvt(unsafe { abi::link(old.as_ptr(), new.as_ptr()) }).map(drop)
}

#[inline]
pub(crate) fn opendir(path: &CStr) -> io::Result<*mut abi::DIR> {
    let dir = unsafe { abi::opendir(path.as_ptr()) };
    if dir.is_null() { Err(io::Error::last_os_error()) } else { Ok(dir) }
}

#[inline]
pub(crate) fn readdir(dir: *mut abi::DIR) -> io::Result<Option<abi::dirent>> {
    crate::sys::io::set_errno(0);
    let entry = unsafe { abi::readdir(dir) };
    if entry.is_null() {
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(0) { Ok(None) } else { Err(error) }
    } else {
        Ok(Some(unsafe { *entry }))
    }
}

#[inline]
pub(crate) fn closedir(dir: *mut abi::DIR) -> io::Result<()> {
    cvt(unsafe { abi::closedir(dir) }).map(drop)
}

#[inline]
pub(crate) fn file_type_from_mode(mode: abi::mode_t) -> u8 {
    match mode & S_IFMT {
        S_IFDIR => DT_DIR,
        S_IFREG => DT_REG,
        S_IFLNK => DT_LNK,
        _ => DT_UNKNOWN,
    }
}

#[inline]
fn cvt(ret: c_int) -> io::Result<c_int> {
    if ret < 0 { Err(io::Error::last_os_error()) } else { Ok(ret) }
}

#[inline]
fn cvt_isize(ret: isize) -> io::Result<isize> {
    if ret < 0 { Err(io::Error::last_os_error()) } else { Ok(ret) }
}
