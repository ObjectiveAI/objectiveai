//! The seven asks a mount makes of the caller, by its id.
//!
//! Each is one [`fuse`] ask on `/requests` and its one-message
//! answer, through [`crate::ask`], made from the FUSE thread over the
//! runtime handle's `block_on`. Every failure of the transport, and
//! every error the caller answers, is `EIO` to the program: what the
//! caller refused is not the program's to know. A read-only mount
//! never asks a mutation: the four that change something answer
//! `EROFS` here, before any ask.

use std::sync::Arc;

use axum::body::Bytes;
use diverge_provider_sdk::container_proxy::fuse;
use diverge_provider_sdk::container_proxy::requests::request::Request;
use fuser::Errno;
use tokio::runtime::Handle;

use crate::ask;
use crate::requests::Requests;

/// One mount's line to the caller.
pub struct Asks {
    requests: Arc<Requests>,
    handle: Handle,
    /// The mount's id, echoed on every ask.
    id: String,
    /// Whether every mutation is refused.
    readonly: bool,
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
    pub fn new(requests: Arc<Requests>, handle: Handle, id: &str, readonly: bool) -> Self {
        Asks {
            requests,
            handle,
            id: id.to_string(),
            readonly,
        }
    }

    /// Whether every mutation is refused.
    pub fn readonly(&self) -> bool {
        self.readonly
    }

    /// One ask, and its one message.
    fn ask(&self, request: Request<'_>) -> Result<Bytes, Errno> {
        self.handle
            .block_on(ask::ask(&self.requests, request))
            .map_err(|_| Errno::EIO)
    }

    /// The gate every mutation passes first.
    fn mutation(&self) -> Result<(), Errno> {
        if self.readonly { Err(Errno::EROFS) } else { Ok(()) }
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
        let request = fuse::stat::request::Request { id: &self.id, path };
        let answer = self.ask(Request::FuseStat(request))?;
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
        let request = fuse::read::request::Request { id: &self.id, path };
        let answer = self.ask(Request::FuseRead(request))?;
        match fuse::read::response::Frame::decode(&answer) {
            Ok(fuse::read::response::Frame::Present(bytes)) => Ok(Some(bytes.to_vec())),
            Ok(fuse::read::response::Frame::Missing) => Ok(None),
            Ok(fuse::read::response::Frame::Error(_)) | Err(_) => Err(Errno::EIO),
        }
    }

    /// A file, stored whole; made if absent.
    pub fn write(&self, path: &str, bytes: &[u8]) -> Result<(), Errno> {
        self.mutation()?;
        let request = fuse::write::request::Request {
            id: &self.id,
            path,
            bytes,
        };
        let answer = self.ask(Request::FuseWrite(request))?;
        Self::ack(&answer)
    }

    /// A directory's entries: `None` when the caller holds no
    /// directory at the path.
    pub fn list(&self, path: &str) -> Result<Option<Vec<Listed>>, Errno> {
        let request = fuse::list::request::Request { id: &self.id, path };
        let answer = self.ask(Request::FuseList(request))?;
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
        self.mutation()?;
        let request = fuse::remove::request::Request { id: &self.id, path };
        let answer = self.ask(Request::FuseRemove(request))?;
        Self::ack(&answer)
    }

    /// An entry, moved within the mount.
    pub fn rename(&self, from: &str, to: &str) -> Result<(), Errno> {
        self.mutation()?;
        let request = fuse::rename::request::Request {
            id: &self.id,
            from,
            to,
        };
        let answer = self.ask(Request::FuseRename(request))?;
        Self::ack(&answer)
    }

    /// A directory, made.
    pub fn mkdir(&self, path: &str) -> Result<(), Errno> {
        self.mutation()?;
        let request = fuse::mkdir::request::Request { id: &self.id, path };
        let answer = self.ask(Request::FuseMkdir(request))?;
        Self::ack(&answer)
    }
}
