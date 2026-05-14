use why2025_badge_sys_bindings as abi;

use crate::ffi::{CStr, OsString, c_int};
use crate::fs::TryLockError;
use crate::hash::Hash;
use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut, SeekFrom};
use crate::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use crate::path::{Path, PathBuf};
pub use crate::sys::fs::common::{Dir, exists, remove_dir_all};
use crate::sys::helpers::run_path_with_cstr;
use crate::sys::pal::fs as pal_fs;
use crate::sys::time::SystemTime;
use crate::sys::{AsInner, AsInnerMut, FromInner, IntoInner, unsupported};

#[derive(Debug)]
pub struct File(OwnedFd);

#[derive(Clone)]
pub struct FileAttr {
    stat: abi::stat,
}

#[derive(Debug)]
pub struct ReadDir {
    root: PathBuf,
    dir: *mut abi::DIR,
}

pub struct DirEntry {
    root: PathBuf,
    entry: abi::dirent,
}

#[derive(Clone, Debug)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct FileTimes {}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FilePermissions {
    mode: abi::mode_t,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct FileType {
    mode: abi::mode_t,
    dirent_type: u8,
}

#[derive(Debug)]
pub struct DirBuilder {}

impl FileAttr {
    pub fn size(&self) -> u64 {
        self.stat.st_size as u64
    }

    pub fn perm(&self) -> FilePermissions {
        FilePermissions { mode: self.stat.st_mode }
    }

    pub fn file_type(&self) -> FileType {
        FileType {
            mode: self.stat.st_mode,
            dirent_type: pal_fs::file_type_from_mode(self.stat.st_mode),
        }
    }

    pub fn modified(&self) -> io::Result<SystemTime> {
        system_time_from_timespec(self.stat.st_mtim)
    }

    pub fn accessed(&self) -> io::Result<SystemTime> {
        system_time_from_timespec(self.stat.st_atim)
    }

    pub fn created(&self) -> io::Result<SystemTime> {
        system_time_from_timespec(self.stat.st_ctim)
    }
}

impl FilePermissions {
    pub fn readonly(&self) -> bool {
        self.mode & 0o222 == 0
    }

    pub fn set_readonly(&mut self, readonly: bool) {
        if readonly {
            self.mode &= !0o222;
        } else {
            self.mode |= 0o200;
        }
    }
}

impl FileTimes {
    pub fn set_accessed(&mut self, _t: SystemTime) {}
    pub fn set_modified(&mut self, _t: SystemTime) {}
}

impl FileType {
    pub fn is_dir(&self) -> bool {
        (self.mode & pal_fs::S_IFMT) == pal_fs::S_IFDIR || self.dirent_type == pal_fs::DT_DIR
    }

    pub fn is_file(&self) -> bool {
        (self.mode & pal_fs::S_IFMT) == pal_fs::S_IFREG || self.dirent_type == pal_fs::DT_REG
    }

    pub fn is_symlink(&self) -> bool {
        (self.mode & pal_fs::S_IFMT) == pal_fs::S_IFLNK || self.dirent_type == pal_fs::DT_LNK
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        match pal_fs::readdir(self.dir) {
            Ok(Some(entry)) => Some(Ok(DirEntry { root: self.root.clone(), entry })),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        }
    }
}

impl Drop for ReadDir {
    fn drop(&mut self) {
        let _ = pal_fs::closedir(self.dir);
    }
}

impl DirEntry {
    pub fn path(&self) -> PathBuf {
        self.root.join(self.file_name())
    }

    pub fn file_name(&self) -> OsString {
        let name = dirent_name(&self.entry).to_vec();
        unsafe { OsString::from_encoded_bytes_unchecked(name) }
    }

    pub fn metadata(&self) -> io::Result<FileAttr> {
        stat(&self.path())
    }

    pub fn file_type(&self) -> io::Result<FileType> {
        Ok(FileType { mode: 0, dirent_type: self.entry.d_type })
    }
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
        }
    }

    pub fn read(&mut self, read: bool) {
        self.read = read;
    }

    pub fn write(&mut self, write: bool) {
        self.write = write;
    }

    pub fn append(&mut self, append: bool) {
        self.append = append;
    }

    pub fn truncate(&mut self, truncate: bool) {
        self.truncate = truncate;
    }

    pub fn create(&mut self, create: bool) {
        self.create = create;
    }

    pub fn create_new(&mut self, create_new: bool) {
        self.create_new = create_new;
    }

    fn flags(&self) -> c_int {
        let mut flags = if self.read && (self.write || self.append) {
            pal_fs::O_RDWR
        } else if self.write || self.append {
            pal_fs::O_WRONLY
        } else {
            pal_fs::O_RDONLY
        };

        if self.append {
            flags |= pal_fs::O_APPEND;
        }
        if self.truncate {
            flags |= pal_fs::O_TRUNC;
        }
        if self.create {
            flags |= pal_fs::O_CREAT;
        }
        if self.create_new {
            flags |= pal_fs::O_CREAT | pal_fs::O_EXCL;
        }

        flags
    }
}

impl File {
    pub fn open(path: &Path, opts: &OpenOptions) -> io::Result<File> {
        run_path_with_cstr(path, &|path| {
            let fd = pal_fs::open(path, opts.flags(), pal_fs::DEFAULT_FILE_MODE)?;
            Ok(unsafe { File::from_raw_fd(fd) })
        })
    }

    pub fn file_attr(&self) -> io::Result<FileAttr> {
        pal_fs::fstat(self.as_raw_fd()).map(|stat| FileAttr { stat })
    }

    pub fn fsync(&self) -> io::Result<()> {
        unsupported()
    }

    pub fn datasync(&self) -> io::Result<()> {
        unsupported()
    }

    pub fn lock(&self) -> io::Result<()> {
        unsupported()
    }

    pub fn lock_shared(&self) -> io::Result<()> {
        unsupported()
    }

    pub fn try_lock(&self) -> Result<(), TryLockError> {
        Err(TryLockError::Error(io::Error::UNSUPPORTED_PLATFORM))
    }

    pub fn try_lock_shared(&self) -> Result<(), TryLockError> {
        Err(TryLockError::Error(io::Error::UNSUPPORTED_PLATFORM))
    }

    pub fn unlock(&self) -> io::Result<()> {
        unsupported()
    }

    pub fn truncate(&self, _size: u64) -> io::Result<()> {
        unsupported()
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        pal_fs::read(self.as_raw_fd(), buf)
    }

    pub fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        io::default_read_vectored(|buf| self.read(buf), bufs)
    }

    pub fn is_read_vectored(&self) -> bool {
        false
    }

    pub fn read_buf(&self, cursor: BorrowedCursor<'_>) -> io::Result<()> {
        io::default_read_buf(|buf| self.read(buf), cursor)
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        pal_fs::write(self.as_raw_fd(), buf)
    }

    pub fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        io::default_write_vectored(|buf| self.write(buf), bufs)
    }

    pub fn is_write_vectored(&self) -> bool {
        false
    }

    pub fn flush(&self) -> io::Result<()> {
        Ok(())
    }

    pub fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        let (offset, whence) = match pos {
            SeekFrom::Start(offset) => {
                (offset.try_into().map_err(|_| invalid_seek())?, pal_fs::SEEK_SET)
            }
            SeekFrom::End(offset) => (offset, pal_fs::SEEK_END),
            SeekFrom::Current(offset) => (offset, pal_fs::SEEK_CUR),
        };
        pal_fs::lseek(self.as_raw_fd(), offset.try_into().map_err(|_| invalid_seek())?, whence)
    }

    pub fn size(&self) -> Option<io::Result<u64>> {
        Some(self.file_attr().map(|attr| attr.size()))
    }

    pub fn tell(&self) -> io::Result<u64> {
        self.seek(SeekFrom::Current(0))
    }

    pub fn duplicate(&self) -> io::Result<File> {
        unsupported()
    }

    pub fn set_permissions(&self, _perm: FilePermissions) -> io::Result<()> {
        unsupported()
    }

    pub fn set_times(&self, _times: FileTimes) -> io::Result<()> {
        unsupported()
    }
}

impl DirBuilder {
    pub fn new() -> DirBuilder {
        DirBuilder {}
    }

    pub fn mkdir(&self, path: &Path) -> io::Result<()> {
        run_path_with_cstr(path, &|path| pal_fs::mkdir(path, pal_fs::DEFAULT_DIR_MODE))
    }
}

pub fn readdir(path: &Path) -> io::Result<ReadDir> {
    run_path_with_cstr(path, &|path_c| {
        let dir = pal_fs::opendir(path_c)?;
        Ok(ReadDir { root: path.to_path_buf(), dir })
    })
}

pub fn unlink(path: &Path) -> io::Result<()> {
    run_path_with_cstr(path, &|path| pal_fs::unlink(path))
}

pub fn rename(old: &Path, new: &Path) -> io::Result<()> {
    run_path_with_cstr(old, &|old| run_path_with_cstr(new, &|new| pal_fs::rename(old, new)))
}

pub fn set_perm(_path: &Path, _perm: FilePermissions) -> io::Result<()> {
    unsupported()
}

pub fn set_times(_path: &Path, _times: FileTimes) -> io::Result<()> {
    unsupported()
}

pub fn set_times_nofollow(_path: &Path, _times: FileTimes) -> io::Result<()> {
    unsupported()
}

pub fn rmdir(path: &Path) -> io::Result<()> {
    run_path_with_cstr(path, &|path| pal_fs::rmdir(path))
}

pub fn readlink(_path: &Path) -> io::Result<PathBuf> {
    unsupported()
}

pub fn symlink(_original: &Path, _link: &Path) -> io::Result<()> {
    unsupported()
}

pub fn link(src: &Path, dst: &Path) -> io::Result<()> {
    run_path_with_cstr(src, &|src| run_path_with_cstr(dst, &|dst| pal_fs::link(src, dst)))
}

pub fn stat(path: &Path) -> io::Result<FileAttr> {
    run_path_with_cstr(path, &|path| pal_fs::stat(path).map(|stat| FileAttr { stat }))
}

pub fn lstat(path: &Path) -> io::Result<FileAttr> {
    stat(path)
}

pub fn canonicalize(_path: &Path) -> io::Result<PathBuf> {
    unsupported()
}

pub fn copy(_from: &Path, _to: &Path) -> io::Result<u64> {
    unsupported()
}

impl AsInner<OwnedFd> for File {
    fn as_inner(&self) -> &OwnedFd {
        &self.0
    }
}

impl AsInnerMut<OwnedFd> for File {
    fn as_inner_mut(&mut self) -> &mut OwnedFd {
        &mut self.0
    }
}

impl IntoInner<OwnedFd> for File {
    fn into_inner(self) -> OwnedFd {
        self.0
    }
}

impl FromInner<OwnedFd> for File {
    fn from_inner(fd: OwnedFd) -> Self {
        Self(fd)
    }
}

impl AsFd for File {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl AsRawFd for File {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl IntoRawFd for File {
    fn into_raw_fd(self) -> RawFd {
        self.0.into_raw_fd()
    }
}

impl FromRawFd for File {
    unsafe fn from_raw_fd(raw_fd: RawFd) -> Self {
        unsafe { Self(OwnedFd::from_raw_fd(raw_fd)) }
    }
}

fn system_time_from_timespec(t: abi::timespec) -> io::Result<SystemTime> {
    SystemTime::new(t.tv_sec as i64, t.tv_nsec as i64)
}

fn dirent_name(entry: &abi::dirent) -> &[u8] {
    let bytes = unsafe {
        crate::slice::from_raw_parts(entry.d_name.as_ptr().cast::<u8>(), entry.d_name.len())
    };
    let len = bytes.iter().position(|byte| *byte == 0).unwrap_or(bytes.len());
    &bytes[..len]
}

fn invalid_seek() -> io::Error {
    io::const_error!(io::ErrorKind::InvalidInput, "invalid seek offset")
}

#[allow(dead_code)]
fn _assert_path_cstr(_: &CStr) {}
