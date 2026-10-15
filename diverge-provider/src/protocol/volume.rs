//! One volume a provider offers, and everything done to it in place.

use std::future::Future;

use bytes::Bytes;
use futures_util::Stream;

use super::hold::{Allow, Held, Holder, Refused};
use super::holders::Holders;
use super::served::Served;
use diverge_sdk::provider::endpoints::volumes::edit::client::request::Change;
use diverge_sdk::provider::endpoints::volumes::edit::server::response::Edit;
use diverge_sdk::provider::endpoints::volumes::stat::server::response::Stat;
use diverge_sdk::provider::endpoints::volumes::Mode;
use diverge_sdk::shared::error::Error;
use diverge_sdk::shared::filetree::response::{Frame, Node};

/// One named directory of one caller, found by
/// [`VolumeManager::get`](super::volume_manager::VolumeManager::get).
///
/// What a [`VolumeManager`](super::volume_manager::VolumeManager)
/// hands back for a name it holds. The verbs that act on an existing
/// volume in place live here — [`stat`](Self::stat),
/// [`read`](Self::read), [`write`](Self::write),
/// [`filetree`](Self::filetree) and [`edit`](Self::edit) — and so
/// does the one fact this crate keeps
/// about a volume for itself: who is using it, which is the hold —
/// [`mount`](Self::mount), shared, and [`lock`](Self::lock),
/// exclusive.
///
/// # Two holds: many mounters, or one editor
///
/// Who may hold a volume at once is its [`Mode`], as far as the
/// provider's capability for the mode goes — see
/// [`Capabilities`](diverge_sdk::provider::endpoints::info::server::response::Capabilities):
/// an ephemeral or a read-only volume may be mounted in any number of
/// containers of its caller and served on any number of scopes at
/// once, a persistent volume in any number of containers but on one
/// scope and never both at once, and a provider may allow less than
/// the mode does. Whatever the mode, nothing
/// examines, reads, writes, walks, resizes or deletes a volume while
/// anything holds it. Both rules are one hold with two kinds, taken
/// by whoever is using the volume, and decided by
/// [`hold`](super::hold) — the one place the rule is written:
///
/// - A run takes the SHARED hold, [`mount`](Self::mount), as a
///   [`Holder::Run`], once on every distinct volume its request names,
///   before it fetches or deploys anything, and keeps every one until
///   the run ends — a stop, the container's own end, the caller going
///   away. Every ending gives them back. A request naming one volume
///   at several paths takes it once. An image build takes it the
///   same way for its context. A
///   [`fuse_serve`](diverge_sdk::provider::endpoints::volumes::fuse_serve)
///   takes it as a [`Holder::FuseServe`] for its scope's life: a
///   served volume is a mounted one, answering the mount's asks
///   itself. The asker says, through `admit`, what the mode it finds
///   allows — the capability's `mount_many` or `fuse_serve_many` as
///   [`Allow::many`], its `mount_and_fuse_serve` as
///   [`Allow::beside`] — or declines the mode outright, and the hold
///   is taken or refused under the volume's own lock, so that an
///   edit cannot land between the look at the mode and the answer.
///   A refusal says who holds the volume, [`Held`], or that the mode
///   was declined, [`Refused::Declined`], and the handler answers the
///   caller in the wire's words: the run's `VolumeHeld`,
///   `VolumeMounted`, `VolumeFuseServed`, `VolumeMountUnsupported` or
///   `VolumeMode`; the FUSE serve's `Held`, `Mounted`, `FuseServed`,
///   `Unsupported` or `VolumeMode`.
/// - A [`stat`](diverge_sdk::provider::endpoints::volumes::stat) takes the EXCLUSIVE
///   hold, [`lock`](Self::lock), for the length of the examination,
///   a [`read`](diverge_sdk::provider::endpoints::volumes::read) for the length of
///   the file, a [`write`](diverge_sdk::provider::endpoints::volumes::write) until
///   the file has landed, a
///   [`filetree`](diverge_sdk::provider::endpoints::volumes::filetree) for the length
///   of the walk, an [`edit`](diverge_sdk::provider::endpoints::volumes::edit) for
///   the length of the change, and a
///   [`delete`](diverge_sdk::provider::endpoints::volumes::delete) takes it and never
///   gives it back, the volume being gone. A volume held at all —
///   mounted anywhere, served, or under another of the six — when any
///   of them asks is that request refused, with the endpoint's own
///   `Mounted`, `FuseServed` or `Held`, as the [`Held`] says.
///
/// There is no verb on a volume that does not take one of the holds,
/// and [`watch`](Self::watch) runs under the shared one: a volume the
/// provider keeps out of a container's tree is watched by the
/// provider itself, through `watch`, and the run handler merges what
/// it reports into the tree the caller sees, which is one tree of
/// every mount however each is watched.
///
/// The handlers do all of that. A provider's hold is a try-hold and
/// nothing more: each method takes the hold or says it cannot, and
/// none waits, because the party that holds it may hold it for the
/// life of a container and a request cannot queue behind that. The
/// methods are futures so a provider may keep the hold wherever it
/// likes — one atomic, a table, a service — but what they await is
/// the provider's own bookkeeping, never another holder. What a hold
/// MEANS — refused, mounted, an error — is decided by the handlers
/// and by [`hold`](super::hold), never by the provider, which is why
/// [`VolumeManager::delete`](super::volume_manager::VolumeManager::delete)
/// answers nothing about mounts.
///
/// # A volume is a handle, not a snapshot
///
/// Whatever the provider holds a volume by: an entry in its own
/// cache, a path, a row. Nothing is read at
/// [`get`](super::volume_manager::VolumeManager::get) time — every
/// method asks the volume as it is now — and a handle may outlive
/// the volume, since a delete on the manager takes a name and not a
/// handle. A method on a handle to a deleted volume fails with the
/// provider's error, which is the same answer a stale name gets.
pub trait Volume: Send + Sync {
    /// Whatever this provider's volumes fail with. The same type its
    /// [`VolumeManager`](super::volume_manager::VolumeManager) fails
    /// with, which that trait's bound states; see
    /// [`VolumeManager::Error`](super::volume_manager::VolumeManager::Error)
    /// for why it is the provider's own.
    type Error: Send + 'static;

    /// Take a shared hold as `holder`, as the volume's mode allows.
    ///
    /// Under the provider's own lock on the volume's bookkeeping, in
    /// this order: the mode read; `admit` asked what that mode allows
    /// the holder — `None` is the mode declined, nothing taken, and
    /// [`Refused::Declined`] with the mode; `Some` is
    /// [`hold::admit`](super::hold::admit) asked with the holders as
    /// they are, which either takes the hold — the holders become
    /// what it returns, and the mode is the answer — or says who
    /// refused, [`Refused::Held`]. The caller keeps a taken hold
    /// until its [`unmount`](Self::unmount) of the same kind, and the
    /// mode does not change meanwhile: an edit needs the exclusive
    /// hold, which the shared one refuses.
    ///
    /// # It never waits
    ///
    /// The exclusive holder may be an edit that resizes for minutes,
    /// and a request that queued behind it would be a request that
    /// never answered. So this answers at once, and what a refusal
    /// means is the handler's to decide; see the trait.
    fn mount(
        &self,
        holder: Holder,
        admit: impl Fn(Mode) -> Option<Allow> + Send,
    ) -> impl Future<Output = Result<Mode, Refused>> + Send;

    /// The mode the volume is in now, as its listing reports it. Read
    /// by the handlers under a hold, which is what keeps an edit from
    /// changing it meanwhile.
    fn mode(&self) -> impl Future<Output = Mode> + Send;

    /// Give one shared hold of `holder`'s kind back, as
    /// [`hold::release`](super::hold::release) decides: `true` is one
    /// holder fewer, and there was one; `false` is a volume with no
    /// such hold to give back, and nothing changed. Called only by a
    /// holder, once per [`mount`](Self::mount) that was taken, and
    /// the handlers keep that count, so a `false` here is a handler's
    /// mistake and not a state a provider has to defend against.
    fn unmount(&self, holder: Holder) -> impl Future<Output = bool> + Send;

    /// Take the exclusive hold, as [`hold::lock`](super::hold::lock)
    /// decides: `Ok` is taken, and the caller keeps it until its
    /// [`unlock`](Self::unlock); `Err` is who has the volume —
    /// a running container, a FUSE serve, or an operation — and
    /// nothing changed. Never waits, as [`mount`](Self::mount) does
    /// not: the holder may be a container that runs for hours.
    fn lock(&self) -> impl Future<Output = Result<(), Held>> + Send;

    /// Give the exclusive hold back, as
    /// [`hold::unlock`](super::hold::unlock) decides: `true` is the
    /// hold released, and it was held; `false` is a hold that was not
    /// held, and nothing changed. Called only by the holder, once per
    /// [`lock`](Self::lock) that was taken.
    fn unlock(&self) -> impl Future<Output = bool> + Send;

    /// Who holds the volume, as of now: nobody, the running
    /// containers and the FUSE serves that have it, or the one
    /// exclusive holder.
    ///
    /// A fact about now and nothing more: [`Free`](Holders::Free) here
    /// is not a promise that the [`lock`](Self::lock) after it is
    /// taken. No handler asks it; it is for the provider's own use —
    /// a listing that wants to say which volumes are in use, a log.
    fn holders(&self) -> impl Future<Output = Holders> + Send;

    /// Who watches a mount of this volume for a caller's filetree:
    /// `true`, the container's proxy, which walks and watches the
    /// mount as any directory of the container; `false`, the provider
    /// itself, through [`watch`](Self::watch). A run handler names the
    /// container path of every mount of a volume that answers `false`
    /// in the tree request it sends the proxy, beside every FUSE
    /// mount's, and opens a `watch` for each instead; the caller
    /// receives one tree and cannot tell which was which. A dataset
    /// the provider offers, large and still, is the case for `false`:
    /// a proxy that walked it would pay for every file on every
    /// filetree, where the provider can watch it once.
    ///
    /// A fact the provider holds, not something it computes, so it is
    /// not `async`.
    fn tree(&self) -> bool;

    /// A watch of the volume's subtree at `path`, as
    /// [`watch`](Self::watch) hands one back: the frames of a
    /// [`filetree`](diverge_sdk::shared::filetree), or the provider's error
    /// where the watch died.
    type Watch: Stream<Item = Result<Frame, Self::Error>> + Send + 'static;

    /// Watch the volume's subtree at `path` — components from the
    /// volume's root, the mount's `volume_relative_path`, empty for the
    /// whole volume — for as long as the stream is held.
    ///
    /// The first frame is a
    /// [`Snapshot`](diverge_sdk::shared::filetree::response::Frame::Snapshot)
    /// of that subtree and every frame after it one change, every
    /// path relative to `path`; a source that lost track sends a
    /// fresh snapshot. A symlink's target is reported as the volume
    /// holds it. An [`Err`] here is a watch that could not be made,
    /// an [`Err`] item a watch that died, and either ends the caller's
    /// whole filetree with the error, the container's tree included:
    /// a tree with a hole in it would be a tree that lied. Called
    /// only while a run holds the volume shared, so nothing resizes
    /// or removes it meanwhile; the stream is dropped when the
    /// caller's channel ends, and nothing else stops it.
    fn watch(&self, path: &[String]) -> impl Future<Output = Result<Self::Watch, Self::Error>> + Send;

    /// The volume, examined: how much of it is used and what is in
    /// it.
    ///
    /// The two fields a listing does not carry, because each is a
    /// walk of the volume — a size to sum, a tree to hash — and a
    /// listing that paid for every volume's walk would pay for the
    /// ones nobody asked about.
    ///
    /// Called under the exclusive hold, so no container writes while
    /// it walks, and what it reports is the volume at rest.
    fn stat(&self) -> impl Future<Output = Result<Stat, Self::Error>> + Send;

    /// A file's bytes, as [`read`](Self::read) hands them back: pieces
    /// in order, each at most [`CHUNK_SIZE`](diverge_sdk::CHUNK_SIZE), or
    /// the provider's error where the read died.
    type Read: Stream<Item = Result<Bytes, Self::Error>> + Send + 'static;

    /// Read the file at `path` — components from the volume's root,
    /// at least one, every one a name, as the handler checked — out
    /// of the volume.
    ///
    /// Called under the exclusive hold, so the file is at rest and
    /// the stream is the whole of it, with nothing writing underneath.
    /// An [`Err`] here is a file that could not be opened — nothing at
    /// the path, a directory there, a link that leads nowhere — and an
    /// [`Err`] item a read that died partway; the handler sends either
    /// as the scope's error, last. A zero-byte file is one empty piece.
    /// Served for every volume the provider offers, whatever it keeps
    /// one in.
    fn read(&self, path: &[String]) -> impl Future<Output = Result<Self::Read, Self::Error>> + Send;

    /// Write `content` to the file at `path` — components from the
    /// volume's root, at least one, every one a name, as the handler
    /// checked — into the volume, whole.
    ///
    /// Called under the exclusive hold, so no container has the volume
    /// while the file lands. Every missing parent directory is made; a
    /// parent that exists and is not a directory is the provider's
    /// error. The destination is replaced whole: it is what it was, or
    /// the new file, and never the half between, however the provider
    /// arranges that — a temporary beside it moved over, a write under
    /// a lock nobody else can take. `content` is the client's pieces
    /// in order, and an [`Err`] item is the client saying it cannot
    /// finish: the write is abandoned, the destination is as it was,
    /// and that error — or the provider's own — is the answer. What a
    /// [`stat`](Self::stat) reported before is stale after a write
    /// that landed, and a provider that caches a walk forgets it.
    fn write<S>(&self, path: &[String], content: S) -> impl Future<Output = Result<(), Self::Error>> + Send
    where
        S: Stream<Item = Result<Bytes, Error>> + Send + 'static;

    /// The volume's subtree at `path` — components from the volume's
    /// root, every one a name, empty for the whole — as the nodes a
    /// [`Snapshot`](diverge_sdk::shared::filetree::response::Frame::Snapshot)
    /// carries, once.
    ///
    /// Called under the exclusive hold, so the tree is the volume at
    /// rest. Every directory's `changes` is `false`, since nothing
    /// watches; a symlink's `path` is its target as components
    /// relative to the volume's root, as a [`watch`](Self::watch)
    /// reports one. Nothing at the path, or a file there, is the
    /// provider's error. Served for every volume the provider offers,
    /// a stored image and a fixed directory alike.
    fn filetree(&self, path: &[String]) -> impl Future<Output = Result<Vec<Node>, Self::Error>> + Send;

    /// The volume answering the FUSE asks, as [`serve`](Self::serve)
    /// hands it back: see [`Served`].
    type Served: Served + 'static;

    /// The volume, served: a value that answers the nine asks of
    /// [`fuse`](diverge_sdk::shared::containers::fuse) from the volume's own
    /// content, in place, for as long as the caller's scope lives.
    ///
    /// Called under the SHARED hold — the handler took it, as a run
    /// does — and the hold outlives what this returns: the handler
    /// gives it back when the scope ends, after the last ask. A
    /// stored volume opens its image once here and answers every ask
    /// on it; a fixed volume answers from its directory. The
    /// [`Mode`] is the provider's to
    /// keep in what it returns: a persistent volume changes in place;
    /// an ephemeral one takes every change into a layer of this
    /// FUSE serve's own, discarded with what this returns, of at most
    /// `overlay_disk` bytes — a provider that cannot set that much
    /// aside answers an error here, and a change past it is refused
    /// with an error while the FUSE serve continues; a read-only one
    /// answers every mutation
    /// [`ReadOnly`](diverge_sdk::shared::containers::fuse::ack::Refused::ReadOnly).
    /// What a [`stat`](Self::stat) reported before is stale after a
    /// serve, and a provider that caches a walk forgets it.
    fn serve(&self, overlay_disk: u64) -> impl Future<Output = Result<Self::Served, Self::Error>> + Send;

    /// Change how big the volume may be, in BYTES, its
    /// [`Mode`], or both.
    ///
    /// The two things that can change, and nothing else. Called under
    /// the exclusive hold, so no container has the volume while its
    /// size or its mode changes — which is what makes the mode
    /// changeable at all, since a container is bound under one mode
    /// for its life and the provider need never move a running one.
    /// A [`Change::Both`] that is refused for its size — no room, or
    /// content that exceeds it — leaves the mode as it was too: the
    /// answer means the volume is as it was in every respect.
    ///
    /// # Why a rename is not an edit
    ///
    /// Because [`name`](diverge_sdk::provider::endpoints::volumes::list::server::response::Volume::name)
    /// is the handle and the only one. Every
    /// [`Mount`](super::mount::Mount) that names a volume names it by
    /// this — so changing it would not modify a volume, it would
    /// replace one with another that nothing outstanding can reach. A
    /// caller that wants a different name makes a volume with it.
    ///
    /// # Two refusals are answers
    ///
    /// A size the provider cannot reserve is
    /// [`Edit::InsufficientCapacity`]. A size below
    /// [`bytes_used`](diverge_sdk::provider::endpoints::volumes::stat::server::response::Stat::bytes_used)
    /// — a volume holding more than it would then reserve — is
    /// [`Edit::ContentTooLarge`]. Neither is an error, and in both the
    /// size is as it was. The provider is what knows its room and the
    /// volume's contents, which is why both answers are the provider's
    /// to give.
    fn edit(&self, change: Change) -> impl Future<Output = Result<Edit, Self::Error>> + Send;
}
