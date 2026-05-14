//! BadgeVMS platform abstraction helpers.
//!
//! Keep raw firmware bindings behind this module so higher-level `std` backends
//! can implement Rust semantics without spreading BadgeVMS ABI details through
//! process, thread, and sync code.

#![deny(unsafe_op_in_unsafe_fn)]

pub use super::unsupported::{abort_internal, cleanup, init, unsupported};

pub(crate) mod process;
