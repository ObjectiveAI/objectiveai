//! FUSE mounts: a single regular file, or a directory tree, mounted
//! on the server's request and kept for the proxy's life, its
//! contents the caller's.
//!
//! The SDK's [`filesystem`](diverge_provider_sdk::container_proxy::filesystem)
//! module states the semantics of both kinds; `file` and
//! `directory` are the filesystems that keep them, `asks` the
//! six exchanges with the caller they are built on — each one
//! [`fuse`](diverge_provider_sdk::container_proxy::fuse) ask on
//! `/requests`, carrying the mount's id — and `handles` the open
//! file handles both keep the same way: a buffer of the handle's own,
//! read whole on open and stored whole on a changed close.
//!
//! FUSE calls a filesystem on the session's own thread, and the
//! caller is asked on the runtime: every ask is the runtime handle's
//! `block_on`, which is what a handle is for from a thread that is
//! not the runtime's. Unix only — the check host is not the
//! container — and [`mount()`] on anything else refuses.

#[cfg(unix)]
mod asks;
#[cfg(unix)]
mod directory;
#[cfg(unix)]
mod file;
#[cfg(unix)]
mod handles;
mod mounts;
mod serve;

use std::io;
use std::sync::Arc;

use diverge_provider_sdk::container_proxy::fuse::mount::request::Request;
use tokio::runtime::Handle;

pub use mounts::*;
pub use serve::*;

use crate::requests::Requests;

/// Which kind of mount: what the server's request said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// One regular file, the mount point the file itself.
    File,
    /// A directory tree, the mount point its root.
    Directory,
}

/// A mount, held: dropping it unmounts.
pub struct Mounted {
    #[cfg(unix)]
    _session: fuser::BackgroundSession,
}

/// Make every missing parent directory, the mount point itself if
/// absent — the file, or the directory — and mount over it, writable:
/// what may change is the caller's to answer, ask by ask.
#[cfg(unix)]
pub fn mount(requests: Arc<Requests>, handle: Handle, mount: &Request, kind: Kind) -> io::Result<Mounted> {
    use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
    use std::path::PathBuf;

    let mut path = PathBuf::from("/");
    path.extend(&mount.path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut config = fuser::Config::default();
    config.mount_options = vec![
        fuser::MountOption::FSName("diverge-fuse".to_string()),
        fuser::MountOption::DefaultPermissions,
        fuser::MountOption::NoAtime,
        fuser::MountOption::RW,
    ];
    config.acl = fuser::SessionACL::All;
    config.n_threads = Some(1);

    let asks = asks::Asks::new(requests, handle, &mount.id);
    let session = match kind {
        Kind::File => {
            std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(false)
                .mode(FILE_MODE)
                .open(&path)?;
            fuser::Session::new(file::MountedFile::new(asks), &path, &config)?.spawn()?
        }
        Kind::Directory => {
            std::fs::create_dir_all(&path)?;
            std::fs::set_permissions(
                &path,
                std::fs::Permissions::from_mode(DIRECTORY_MODE),
            )?;
            fuser::Session::new(directory::MountedDirectory::new(asks), &path, &config)?.spawn()?
        }
    };
    Ok(Mounted { _session: session })
}

/// Not the container: there is no FUSE here, and a mount is refused.
#[cfg(not(unix))]
pub fn mount(_requests: Arc<Requests>, _handle: Handle, mount: &Request, _kind: Kind) -> io::Result<Mounted> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        format!("FUSE mounts need FUSE, which this host has not: {}", mount.id),
    ))
}

/// A mounted file's mode: owner read and write.
#[cfg(unix)]
pub const FILE_MODE: u32 = 0o600;

/// A mounted directory's mode: owner read, write and search.
#[cfg(unix)]
pub const DIRECTORY_MODE: u32 = 0o700;
