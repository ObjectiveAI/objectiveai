//! The seven asks a mount makes of the caller, on the mount's own
//! scope.
//!
//! Each is one channel request on the scope the mount was made on —
//! the scope is the mount, so no ask names it — and its one-frame
//! answer, through [`crate::ask`], made from the FUSE thread over the
//! runtime handle's `block_on`. Every failure of the transport, and
//! every error the caller answers, is `EIO` to the program: what the
//! caller refused is not the program's to know, and nothing here
//! refuses anything on the caller's behalf.

use std::sync::Arc;

use bytes::Bytes;
use diverge_provider_sdk::container_proxy_endpoints::fuse::mount::server::channel_request::{Frame, Path, Rename, Write};
use diverge_provider_sdk::server::scope_handle::ScopeHandle;
use diverge_provider_sdk::shared::containers::fuse;
use fuser::Errno;
use tokio::runtime::Handle;

use crate::ask;
use crate::encode::encoded;

/// One mount's line to the caller.
pub struct Asks {
    scope: Arc<ScopeHandle>,
    handle: Handle,
}

/// One entry of a listed directory, owned: what a `readdir` shows.
pub struct Listed {
    /// The entry's name.
    pub name: String,
    /// Whether it is a directory.
    pub directory: bool,
}

/// What an entry is, as the caller last said.
pub enum Stat {
    /// A regular file of this length.
    File(u64),
    Directory,
}

impl Asks {
    pub fn new(scope: Arc<ScopeHandle>, handle: Handle) -> Self {
        Asks { scope, handle }
    }

    /// One ask, and its one frame.
    fn ask(&self, frame: Frame<'_>) -> Result<Bytes, Errno> {
        let payload = encoded(&frame).ok_or(Errno::EIO)?;
        self.handle
            .block_on(ask::ask(&self.scope, &payload))
            .map_err(|_| Errno::EIO)
    }

    /// An ok-or-error answer, read.
    fn ack(answer: &[u8]) -> Result<(), Errno> {
        match fuse::ack::Frame::decode(answer) {
            Ok(fuse::ack::Frame::Ok) => Ok(()),
            Ok(fuse::ack::Frame::Error(_)) | Err(_) => Err(Errno::EIO),
        }
    }

    /// What is at the path: `None` when the caller holds nothing
    /// there. Nine bytes back, never the file: this is what every
    /// `getattr` and `lookup` costs.
    pub fn stat(&self, path: &str) -> Result<Option<Stat>, Errno> {
        let answer = self.ask(Frame::Stat(Path { path }))?;
        match fuse::stat::response::Frame::decode(&answer) {
            Ok(fuse::stat::response::Frame::Present(stat)) => Ok(Some(match stat.kind {
                fuse::Kind::File => Stat::File(stat.size),
                fuse::Kind::Directory => Stat::Directory,
            })),
            Ok(fuse::stat::response::Frame::Missing) => Ok(None),
            Ok(fuse::stat::response::Frame::Error(_)) | Err(_) => Err(Errno::EIO),
        }
    }

    /// A file's bytes: `None` when the caller holds nothing at the
    /// path.
    pub fn read(&self, path: &str) -> Result<Option<Vec<u8>>, Errno> {
        let answer = self.ask(Frame::Read(Path { path }))?;
        match fuse::read::response::Frame::decode(&answer) {
            Ok(fuse::read::response::Frame::Present(bytes)) => Ok(Some(bytes.to_vec())),
            Ok(fuse::read::response::Frame::Missing) => Ok(None),
            Ok(fuse::read::response::Frame::Error(_)) | Err(_) => Err(Errno::EIO),
        }
    }

    /// A file, stored whole; made if absent.
    pub fn write(&self, path: &str, bytes: &[u8]) -> Result<(), Errno> {
        let answer = self.ask(Frame::Write(Write { path, bytes }))?;
        Self::ack(&answer)
    }

    /// A directory's entries: `None` when the caller holds no
    /// directory at the path.
    pub fn list(&self, path: &str) -> Result<Option<Vec<Listed>>, Errno> {
        let answer = self.ask(Frame::List(Path { path }))?;
        match fuse::list::response::Frame::decode(&answer) {
            Ok(fuse::list::response::Frame::Entries(entries)) => Ok(Some(
                entries
                    .into_iter()
                    .map(|entry| Listed {
                        name: entry.name.to_string(),
                        directory: entry.kind == fuse::Kind::Directory,
                    })
                    .collect(),
            )),
            Ok(fuse::list::response::Frame::Missing) => Ok(None),
            Ok(fuse::list::response::Frame::Error(_)) | Err(_) => Err(Errno::EIO),
        }
    }

    /// A file or an empty directory, removed.
    pub fn remove(&self, path: &str) -> Result<(), Errno> {
        let answer = self.ask(Frame::Remove(Path { path }))?;
        Self::ack(&answer)
    }

    /// An entry, moved within the mount.
    pub fn rename(&self, from: &str, to: &str) -> Result<(), Errno> {
        let answer = self.ask(Frame::Rename(Rename { from, to }))?;
        Self::ack(&answer)
    }

    /// A directory, made.
    pub fn mkdir(&self, path: &str) -> Result<(), Errno> {
        let answer = self.ask(Frame::Mkdir(Path { path }))?;
        Self::ack(&answer)
    }
}
