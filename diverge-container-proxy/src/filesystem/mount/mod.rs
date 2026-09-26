//! FUSE mounts: a single regular file, or a directory tree, mounted
//! on the server's request and kept for the proxy's life, its
//! contents the caller's.
//!
//! The SDK's [`fuse`](diverge_provider_sdk::shared::containers::fuse)
//! module states the semantics of both kinds; `file` and
//! `directory` are the filesystems that keep them, `asks` the nine
//! exchanges with the caller they are built on — each one channel
//! request on the scope the mount was made on — and `handles` the
//! open file handles both keep the same way: a path and whether it
//! was opened for writing, and nothing else. Nothing is buffered
//! anywhere: every `read(2)` and every `write(2)` the kernel hands a
//! mount is one ask with that call's own offset and size, carried as
//! it comes, and every attribute is the caller's current answer. The
//! mount is opened direct, so the kernel keeps no page cache over
//! it either.
//!
//! FUSE calls a filesystem on the session's own thread, and the
//! caller is asked on the runtime: every ask is the runtime handle's
//! `block_on`, which is what a handle is for from a thread that is
//! not the runtime's. The proxy is Unix, as the container is; nothing
//! here is built for anything else.

mod asks;
mod attrs;
mod directory;
mod file;
mod handles;
mod mounts;

use std::io;
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::Path;
use std::sync::Arc;

use diverge_provider_sdk::server::scope_handle::ScopeHandle;
use tokio::runtime::Handle;

pub use mounts::*;

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
    _session: fuser::BackgroundSession,
}

/// Make every missing parent directory, the mount point itself if
/// absent — the file, or the directory — and mount over it, writable:
/// what may change is the caller's to answer, ask by ask, on `scope`.
/// The modes below are the mount point's own, under the mount; what
/// a program sees is the caller's stat.
pub fn mount(scope: Arc<ScopeHandle>, handle: Handle, path: &Path, kind: Kind) -> io::Result<Mounted> {
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

    let asks = asks::Asks::new(scope, handle);
    let session = match kind {
        Kind::File => {
            std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(false)
                .mode(FILE_MODE)
                .open(path)?;
            fuser::Session::new(file::MountedFile::new(asks), path, &config)?.spawn()?
        }
        Kind::Directory => {
            std::fs::create_dir_all(path)?;
            std::fs::set_permissions(
                path,
                std::fs::Permissions::from_mode(DIRECTORY_MODE),
            )?;
            fuser::Session::new(directory::MountedDirectory::new(asks), path, &config)?.spawn()?
        }
    };
    Ok(Mounted { _session: session })
}

/// A mounted file's mode: owner read and write.
pub const FILE_MODE: u32 = 0o600;

/// A mounted directory's mode: owner read, write and search.
pub const DIRECTORY_MODE: u32 = 0o700;
