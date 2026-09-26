//! The files and directories a caller serves live into a container.

use std::future::Future;

use bytes::Bytes;

pub use crate::shared::containers::fuse::Listed;
use crate::shared::containers::fuse::Attrs;
use crate::shared::containers::fuse::ack::Refused;
use crate::shared::containers::fuse::stat::Stat;

/// The caller's side of [`fuse`](crate::shared::containers::fuse):
/// what stands behind every
/// [`FuseMount`](crate::shared::containers::request::FuseMount) the
/// caller named, answered by the mount's id and an entry's path
/// relative to the mount root — empty for a file mount and for a
/// directory mount's root.
///
/// Nothing is buffered. A [`read`](Self::read) is one piece at an
/// offset, a [`write`](Self::write) lands one piece in place, a
/// [`truncate`](Self::truncate) sets the length, and the caller's
/// storage changes as the asks arrive — a program's `write(2)` is
/// this call, not a close later. What an entry IS is a
/// [`stat`](Self::stat): fifty-seven bytes, the kind, the size, the
/// mode, the owner, the group and the times, asked far more often
/// than the bytes — on every `stat(2)`, every path component, every
/// attribute — so it must not cost them; [`setattr`](Self::setattr)
/// changes the ones a program may. An `Err` of a read, a list or a
/// stat is the text the wire's error carries. `Ok(None)` from `read`
/// and from `stat` is a file the caller holds nothing for yet (it
/// reads as empty, and the first write makes it), or an entry that is
/// not there; from [`list`](Self::list) it is no such directory. A
/// mutation answers [`Refused`]: [`Ephemeral`](Refused::Ephemeral)
/// when the storage keeps nothing written into it, so the program
/// sees a read-only filesystem, or the error in words. A caller that
/// wants a mount unchangeable answers every mutation for its id with
/// the error: nothing else refuses one on its behalf.
pub trait FuseServer: Send + Sync {
    /// What the entry is, how long, whose, with what mode, and when;
    /// or `None` for one not held.
    fn stat(&self, id: &str, path: &str) -> impl Future<Output = Result<Option<Stat>, String>> + Send;

    /// At most `length` bytes of the file from `offset` — fewer at
    /// the end, none at or past it — or `None` for a file not held.
    fn read(&self, id: &str, path: &str, offset: u64, length: u32) -> impl Future<Output = Result<Option<Bytes>, String>> + Send;

    /// The piece landed in place at `offset`; the file made if absent
    /// and extended with zeros to the offset if short.
    fn write(&self, id: &str, path: &str, offset: u64, bytes: Bytes) -> impl Future<Output = Result<(), Refused>> + Send;

    /// The file set to `size` bytes, cut or extended with zeros; made
    /// if absent.
    fn truncate(&self, id: &str, path: &str, size: u64) -> impl Future<Output = Result<(), Refused>> + Send;

    /// The attributes set that `attrs` sets, the rest left; a caller
    /// with no owners to speak of answers ok and keeps what it has.
    fn setattr(&self, id: &str, path: &str, attrs: Attrs) -> impl Future<Output = Result<(), Refused>> + Send;

    /// The directory's entries — names and kinds — or `None` for no
    /// such directory.
    fn list(&self, id: &str, path: &str) -> impl Future<Output = Result<Option<Vec<Listed>>, String>> + Send;

    /// A file or an empty directory, removed; a non-empty directory
    /// is the caller's to refuse.
    fn remove(&self, id: &str, path: &str) -> impl Future<Output = Result<(), Refused>> + Send;

    /// An entry moved within the mount, replacing a file at `to`; a
    /// directory at `to` is the caller's to refuse.
    fn rename(&self, id: &str, from: &str, to: &str) -> impl Future<Output = Result<(), Refused>> + Send;

    /// A directory made under an existing one.
    fn mkdir(&self, id: &str, path: &str) -> impl Future<Output = Result<(), Refused>> + Send;
}
