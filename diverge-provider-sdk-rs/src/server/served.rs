//! A volume, served: the FUSE asks answered from it.

use std::future::Future;

use bytes::Bytes;

use crate::shared::containers::fuse::ack::Refused;
use crate::shared::containers::fuse::stat::Stat;
use crate::shared::containers::fuse::{Attrs, Listed};

/// What [`Volume::serve`](super::volume::Volume::serve) hands back: a
/// volume answering the nine asks of
/// [`fuse`](crate::shared::containers::fuse) from its own content,
/// for as long as a [`serve`](crate::endpoints::volumes::serve) scope
/// lives.
///
/// The caller's [`FuseServer`](crate::client::FuseServer) with the
/// mount's id left off, since the scope is the volume: the same nine
/// methods, the same answers, the same meanings, every path
/// `/`-separated from the volume's root and empty for the root
/// itself. Nothing is buffered: a read is one piece at an offset, a
/// write lands one piece in place, and the volume changes as the asks
/// arrive. Called under the SHARED hold, the one a run takes, so
/// nothing examines, resizes or removes the volume meanwhile and any
/// number of serves and runs may hold it at once.
///
/// # The mode is the provider's to keep
///
/// The handler does not know the volume's
/// [`Mode`](crate::endpoints::volumes::Mode); what this returns does.
/// A persistent volume changes in place. An ephemeral one takes
/// every mutation into a layer of this serve's own, discarded when
/// this is dropped, and refuses with [`Refused::Error`] a mutation
/// that would take the layer past the `overlay_disk` the serve was
/// given. A read-only one answers every mutating method —
/// [`write`](Self::write), [`truncate`](Self::truncate),
/// [`setattr`](Self::setattr), [`remove`](Self::remove),
/// [`rename`](Self::rename), [`mkdir`](Self::mkdir) — with
/// [`Refused::ReadOnly`], and every immutable one goes through.
pub trait Served: Send + Sync {
    /// What the entry is, how long, whose, with what mode, and when;
    /// or `None` for nothing at the path.
    fn stat(&self, path: &str) -> impl Future<Output = Result<Option<Stat>, String>> + Send;

    /// At most `length` bytes of the file from `offset` — fewer at
    /// the end, none at or past it — or `None` for nothing at the
    /// path.
    fn read(&self, path: &str, offset: u64, length: u32) -> impl Future<Output = Result<Option<Bytes>, String>> + Send;

    /// The piece landed in place at `offset`; the file made if absent
    /// and extended with zeros to the offset if short.
    fn write(&self, path: &str, offset: u64, bytes: Bytes) -> impl Future<Output = Result<(), Refused>> + Send;

    /// The file set to `size` bytes, cut or extended with zeros; made
    /// if absent.
    fn truncate(&self, path: &str, size: u64) -> impl Future<Output = Result<(), Refused>> + Send;

    /// The attributes set that `attrs` sets, the rest left.
    fn setattr(&self, path: &str, attrs: Attrs) -> impl Future<Output = Result<(), Refused>> + Send;

    /// The directory's entries — names and kinds — or `None` for no
    /// such directory.
    fn list(&self, path: &str) -> impl Future<Output = Result<Option<Vec<Listed>>, String>> + Send;

    /// A file or an empty directory, removed; a non-empty directory
    /// is refused.
    fn remove(&self, path: &str) -> impl Future<Output = Result<(), Refused>> + Send;

    /// An entry moved within the volume, replacing a file at `to`; a
    /// directory at `to` is refused.
    fn rename(&self, from: &str, to: &str) -> impl Future<Output = Result<(), Refused>> + Send;

    /// A directory made under an existing one.
    fn mkdir(&self, path: &str) -> impl Future<Output = Result<(), Refused>> + Send;
}
