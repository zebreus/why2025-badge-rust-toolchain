//! BadgeVMS-specific extensions.

#![stable(feature = "rust1", since = "1.0.0")]

pub mod io {
    //! BadgeVMS file-descriptor I/O types and traits.

    #![stable(feature = "os_fd", since = "1.66.0")]

    #[stable(feature = "os_fd", since = "1.66.0")]
    pub use crate::os::fd::*;
}

pub mod raw;