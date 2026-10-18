//! The nine asks a mount makes of the caller, on the mount's own
//! scope.
//!
//! Each is one channel request on the scope the mount was made on —
//! the scope is the mount, so no ask names it — and its one-frame
//! answer, through [`crate::ask`], made from the FUSE thread over the
//! runtime handle's `block_on`. Every failure of the transport, and
//! every error the caller answers, is `EIO` to the program; a
//! mutation the caller refuses because the volume is read only is
//! `EROFS`; nothing at the path is `ENOENT`. What the caller refused
//! is not the program's to know, and nothing here refuses anything on
//! the caller's behalf.

use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::container_proxy::outside::endpoints::fuse::mount::server::channel_request::{Frame, Path, Read, Rename, Setattr, Truncate, Write};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::containers::fuse;
use diverge_sdk::shared::containers::fuse::stat::Stat;
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

    /// An ok-or-not answer, read: ok, or the errno the refusal is.
    fn ack(answer: &[u8]) -> Result<(), Errno> {
        match fuse::ack::Frame::decode(answer) {
            Ok(fuse::ack::Frame::Ok) => Ok(()),
            Ok(fuse::ack::Frame::ReadOnly) => Err(Errno::EROFS),
            Ok(fuse::ack::Frame::Error(_)) | Err(_) => Err(Errno::EIO),
        }
    }

    /// What is at the path: `None` when the caller holds nothing
    /// there. Fifty-seven bytes back, never the file: this is what
    /// every `getattr` and `lookup` costs.
    pub fn stat(&self, path: &str) -> Result<Option<Stat>, Errno> {
        let answer = self.ask(Frame::Stat(Path { path }))?;
        match fuse::stat::response::Frame::decode(&answer) {
            Ok(fuse::stat::response::Frame::Present(stat)) => Ok(Some(stat)),
            Ok(fuse::stat::response::Frame::Missing) => Ok(None),
            Ok(fuse::stat::response::Frame::Error(_)) | Err(_) => Err(Errno::EIO),
        }
    }

    /// At most `length` bytes of a file from `offset`: `None` when the
    /// caller holds nothing at the path.
    pub fn read(&self, path: &str, offset: u64, length: u32) -> Result<Option<Bytes>, Errno> {
        let answer = self.ask(Frame::Read(Read { path, offset, length }))?;
        match fuse::read::response::Frame::decode(&answer) {
            Ok(fuse::read::response::Frame::Present(bytes)) => Ok(Some(answer.slice_ref(bytes))),
            Ok(fuse::read::response::Frame::Missing) => Ok(None),
            Ok(fuse::read::response::Frame::Error(_)) | Err(_) => Err(Errno::EIO),
        }
    }

    /// A piece landed in place at `offset`; the file made if absent.
    pub fn write(&self, path: &str, offset: u64, bytes: &[u8]) -> Result<(), Errno> {
        let answer = self.ask(Frame::Write(Write { path, offset, bytes }))?;
        Self::ack(&answer)
    }

    /// A file set to `size` bytes.
    pub fn truncate(&self, path: &str, size: u64) -> Result<(), Errno> {
        let answer = self.ask(Frame::Truncate(Truncate { path, size }))?;
        Self::ack(&answer)
    }

    /// Some of an entry's attributes set.
    pub fn setattr(&self, path: &str, attrs: fuse::Attrs) -> Result<(), Errno> {
        let answer = self.ask(Frame::Setattr(Setattr { path, attrs }))?;
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
