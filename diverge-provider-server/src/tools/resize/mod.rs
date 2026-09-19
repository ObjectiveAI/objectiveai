//! Resizing the filesystem inside an image, with the system's
//! e2fsprogs.
//!
//! Nothing in this crate can resize an ext4 filesystem: the
//! formatter cannot, and podman has no such verb. `resize2fs` can,
//! offline, on the image file itself, with no loop device and no
//! privilege — and it refuses a filesystem that was mounted since it
//! was last checked, so `e2fsck` runs first. Both are assumed to be
//! where this host can reach them: on Linux, on the provider's own
//! `PATH`, run natively; on macOS and Windows, inside the podman
//! machine, which is Fedora CoreOS and ships them, run through
//! [`podman`](super::podman) with the image's path as the machine
//! sees it. [`resize`] is the one call.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod resize;

pub use resize::*;
