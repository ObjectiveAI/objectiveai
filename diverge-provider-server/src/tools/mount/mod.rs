//! Putting the filesystem in a volume's image on a directory, with
//! the system's `mount`.
//!
//! Podman binds directories, not image files, so a stored volume is
//! loop-mounted by the provider before podman is asked, and unmounted
//! after the container is gone. On Linux the provider runs as root
//! and `mount` and `umount` are its own; on macOS and Windows the
//! directory and the mount are the podman machine's, made through
//! [`podman`](super::podman) with the image's path as the machine
//! sees it, since a bind source podman is handed is a path of the
//! machine's. [`mount`] and [`unmount`] are the two calls.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod mount;

pub use mount::*;
