//! BadgeVMS platform abstraction helpers.
//!
//! Keep raw firmware bindings behind this module so higher-level `std` backends
//! can implement Rust semantics without spreading BadgeVMS ABI details through
//! process, thread, and sync code.

#![deny(unsafe_op_in_unsafe_fn)]

pub use super::unsupported::{abort_internal, cleanup, unsupported};

pub(crate) mod fs;
pub(crate) mod process;
pub(crate) mod random;
pub(crate) mod time;

/// # Safety
///
/// Must be called only once during runtime initialization with the raw BadgeVMS
/// process argument vector supplied by the loader/runtime entry path.
pub unsafe fn init(argc: isize, argv: *const *const u8, _sigpipe: u8) {
    unsafe { crate::sys::args::init(argc, argv) };
}
