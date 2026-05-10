//! BadgeVMS raw type aliases.

#![stable(feature = "raw_ext", since = "1.1.0")]
#![deprecated(
    since = "1.8.0",
    note = "these type aliases are no longer supported by the standard library"
)]
#![allow(deprecated)]

#[stable(feature = "raw_ext", since = "1.1.0")]
#[allow(non_camel_case_types)]
pub type blkcnt_t = i64;
#[stable(feature = "raw_ext", since = "1.1.0")]
#[allow(non_camel_case_types)]
pub type blksize_t = i32;
#[stable(feature = "raw_ext", since = "1.1.0")]
#[allow(non_camel_case_types)]
pub type dev_t = u64;
#[stable(feature = "raw_ext", since = "1.1.0")]
#[allow(non_camel_case_types)]
pub type ino_t = u64;
#[stable(feature = "raw_ext", since = "1.1.0")]
#[allow(non_camel_case_types)]
pub type mode_t = u32;
#[stable(feature = "raw_ext", since = "1.1.0")]
#[allow(non_camel_case_types)]
pub type nlink_t = u32;
#[stable(feature = "raw_ext", since = "1.1.0")]
#[allow(non_camel_case_types)]
pub type off_t = i64;
#[stable(feature = "pthread_t", since = "1.8.0")]
#[allow(non_camel_case_types)]
pub type pthread_t = usize;
#[stable(feature = "raw_ext", since = "1.1.0")]
#[allow(non_camel_case_types)]
pub type time_t = i64;
