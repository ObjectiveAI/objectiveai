//! The files and directories a caller serves live into a container.

use std::future::Future;

use bytes::Bytes;

use crate::shared::containers::fuse::Kind;
use crate::shared::containers::fuse::stat::Stat;

/// One entry of a listed directory, owned: what a `readdir` shows.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Listed {
    /// The entry's name, one path component.
    pub name: String,
    /// File or directory.
    pub kind: Kind,
}

/// The caller's side of [`fuse`](crate::shared::containers::fuse):
/// what stands behind every
/// [`FuseMount`](crate::shared::containers::request::FuseMount) the
/// caller named, answered by the mount's id and an entry's path
/// relative to the mount root — empty for a file mount and for a
/// directory mount's root.
///
/// A file is one message either way: read whole, written whole. What
/// it IS is nine bytes: [`stat`](Self::stat) answers the kind and the
/// size, and is asked far more often than [`read`](Self::read) — on
/// every `stat(2)`, every path component, every attribute — so it
/// must not cost the bytes. An `Err` is the text the wire's error
/// carries. `Ok(None)` from `read` and from `stat` is a file the
/// caller holds nothing for yet (it reads as empty, and the first
/// write makes it), or an entry that is not there; from
/// [`list`](Self::list) it is no such directory. A caller that
/// wants a mount unchangeable answers every mutation for its id with
/// the error: nothing else refuses one on its behalf.
pub trait FuseServer: Send + Sync {
    /// What the entry is and how long, or `None` for one not held.
    fn stat(&self, id: &str, path: &str) -> impl Future<Output = Result<Option<Stat>, String>> + Send;

    /// The file's bytes, or `None` for one not held.
    fn read(&self, id: &str, path: &str) -> impl Future<Output = Result<Option<Bytes>, String>> + Send;

    /// The file, stored whole; made if absent.
    fn write(&self, id: &str, path: &str, bytes: Bytes) -> impl Future<Output = Result<(), String>> + Send;

    /// The directory's entries — names and kinds — or `None` for no
    /// such directory.
    fn list(&self, id: &str, path: &str) -> impl Future<Output = Result<Option<Vec<Listed>>, String>> + Send;

    /// A file or an empty directory, removed; a non-empty directory
    /// is the caller's to refuse.
    fn remove(&self, id: &str, path: &str) -> impl Future<Output = Result<(), String>> + Send;

    /// An entry moved within the mount, replacing a file at `to`; a
    /// directory at `to` is the caller's to refuse.
    fn rename(&self, id: &str, from: &str, to: &str) -> impl Future<Output = Result<(), String>> + Send;

    /// A directory made under an existing one.
    fn mkdir(&self, id: &str, path: &str) -> impl Future<Output = Result<(), String>> + Send;
}
