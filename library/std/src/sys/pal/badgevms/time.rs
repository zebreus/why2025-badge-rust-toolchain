use core::num::niche_types::Nanoseconds;

use why2025_badge_sys_bindings as abi;

use crate::io;
use crate::mem::MaybeUninit;
use crate::time::Duration;

const NSEC_PER_SEC: u64 = 1_000_000_000;

pub(crate) const CLOCK_REALTIME: abi::clockid_t = 0;
pub(crate) const CLOCK_MONOTONIC: abi::clockid_t = 1;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub(crate) struct Timespec {
    pub(crate) tv_sec: i64,
    pub(crate) tv_nsec: Nanoseconds,
}

impl Timespec {
    pub(crate) const MAX: Timespec = unsafe { Self::new_unchecked(i64::MAX, 1_000_000_000 - 1) };
    pub(crate) const MIN: Timespec = unsafe { Self::new_unchecked(i64::MIN, 0) };

    pub(crate) const unsafe fn new_unchecked(tv_sec: i64, tv_nsec: i64) -> Timespec {
        Timespec { tv_sec, tv_nsec: unsafe { Nanoseconds::new_unchecked(tv_nsec as u32) } }
    }

    pub(crate) const fn zero() -> Timespec {
        unsafe { Self::new_unchecked(0, 0) }
    }

    pub(crate) const fn new(tv_sec: i64, tv_nsec: i64) -> Result<Timespec, io::Error> {
        if tv_nsec >= 0 && tv_nsec < NSEC_PER_SEC as i64 {
            Ok(unsafe { Self::new_unchecked(tv_sec, tv_nsec) })
        } else {
            Err(io::const_error!(io::ErrorKind::InvalidData, "invalid timestamp"))
        }
    }

    pub(crate) fn now(clock: abi::clockid_t) -> io::Result<Timespec> {
        let mut t = MaybeUninit::uninit();
        cvt(unsafe { abi::clock_gettime(clock, t.as_mut_ptr()) })?;
        let t = unsafe { t.assume_init() };
        Timespec::new(t.tv_sec as i64, t.tv_nsec as i64)
    }

    pub(crate) fn wall_time() -> io::Result<Timespec> {
        match Self::now(CLOCK_REALTIME) {
            Ok(time) => Ok(time),
            Err(_) => Self::gettimeofday(),
        }
    }

    fn gettimeofday() -> io::Result<Timespec> {
        let mut t = MaybeUninit::uninit();
        cvt(unsafe { abi::gettimeofday(t.as_mut_ptr(), crate::ptr::null_mut()) })?;
        let t = unsafe { t.assume_init() };
        Timespec::new(t.tv_sec as i64, (t.tv_usec as i64) * 1_000)
    }

    pub(crate) fn sub_timespec(&self, other: &Timespec) -> Result<Duration, Duration> {
        fn sub_ge_to_unsigned(a: i64, b: i64) -> u64 {
            debug_assert!(a >= b);
            a.wrapping_sub(b).cast_unsigned()
        }

        if self >= other {
            let (secs, nsec) = if self.tv_nsec.as_inner() >= other.tv_nsec.as_inner() {
                (
                    sub_ge_to_unsigned(self.tv_sec, other.tv_sec),
                    self.tv_nsec.as_inner() - other.tv_nsec.as_inner(),
                )
            } else {
                debug_assert!(self.tv_nsec < other.tv_nsec);
                debug_assert!(self.tv_sec > other.tv_sec);
                debug_assert!(self.tv_sec > i64::MIN);
                (
                    sub_ge_to_unsigned(self.tv_sec - 1, other.tv_sec),
                    self.tv_nsec.as_inner() + (NSEC_PER_SEC as u32) - other.tv_nsec.as_inner(),
                )
            };

            Ok(Duration::new(secs, nsec))
        } else {
            match other.sub_timespec(self) {
                Ok(d) => Err(d),
                Err(d) => Ok(d),
            }
        }
    }

    pub(crate) fn checked_add_duration(&self, other: &Duration) -> Option<Timespec> {
        let mut secs = self.tv_sec.checked_add_unsigned(other.as_secs())?;
        let mut nsec = other.subsec_nanos() + self.tv_nsec.as_inner();
        if nsec >= NSEC_PER_SEC as u32 {
            nsec -= NSEC_PER_SEC as u32;
            secs = secs.checked_add(1)?;
        }
        Some(unsafe { Self::new_unchecked(secs, nsec.into()) })
    }

    pub(crate) fn checked_sub_duration(&self, other: &Duration) -> Option<Timespec> {
        let mut secs = self.tv_sec.checked_sub_unsigned(other.as_secs())?;
        let mut nsec = self.tv_nsec.as_inner() as i32 - other.subsec_nanos() as i32;
        if nsec < 0 {
            nsec += NSEC_PER_SEC as i32;
            secs = secs.checked_sub(1)?;
        }
        Some(unsafe { Self::new_unchecked(secs, nsec.into()) })
    }
}

fn cvt(ret: crate::ffi::c_int) -> io::Result<crate::ffi::c_int> {
    if ret < 0 { Err(io::Error::last_os_error()) } else { Ok(ret) }
}
