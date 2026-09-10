//! FUSE mounts: a single regular file, or a directory tree, mounted
//! at the proxy's start, its contents the caller's.
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

use std::io;
use std::sync::Arc;

use diverge_provider_sdk::container_proxy::filesystem::Mount;
use tokio::runtime::Handle;

use crate::requests::Requests;

/// Which kind of mount: which list of the server's it was on.
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
/// absent — the file, or the directory — and mount over it:
/// read-only at the kernel too when the mount says so.
#[cfg(unix)]
pub fn mount(requests: Arc<Requests>, handle: Handle, mount: &Mount, kind: Kind) -> io::Result<Mounted> {
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
        if mount.readonly {
            fuser::MountOption::RO
        } else {
            fuser::MountOption::RW
        },
    ];
    config.acl = fuser::SessionACL::All;
    config.n_threads = Some(1);

    let asks = asks::Asks::new(requests, handle, &mount.id, mount.readonly);
    let session = match kind {
        Kind::File => {
            std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(false)
                .mode(file_mode(mount.readonly))
                .open(&path)?;
            fuser::Session::new(file::MountedFile::new(asks), &path, &config)?.spawn()?
        }
        Kind::Directory => {
            std::fs::create_dir_all(&path)?;
            std::fs::set_permissions(
                &path,
                std::fs::Permissions::from_mode(directory_mode(mount.readonly)),
            )?;
            fuser::Session::new(directory::MountedDirectory::new(asks), &path, &config)?.spawn()?
        }
    };
    Ok(Mounted { _session: session })
}

/// Not the container: there is no FUSE here, and a mount is refused.
#[cfg(not(unix))]
pub fn mount(_requests: Arc<Requests>, _handle: Handle, mount: &Mount, _kind: Kind) -> io::Result<Mounted> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        format!("FUSE mounts need FUSE, which this host has not: {}", mount.id),
    ))
}

/// A mounted file's mode: owner read, and write unless read-only.
#[cfg(unix)]
fn file_mode(readonly: bool) -> u32 {
    if readonly { 0o400 } else { 0o600 }
}

/// A mounted directory's mode: owner read and search, and write
/// unless read-only.
#[cfg(unix)]
fn directory_mode(readonly: bool) -> u32 {
    if readonly { 0o500 } else { 0o700 }
}
