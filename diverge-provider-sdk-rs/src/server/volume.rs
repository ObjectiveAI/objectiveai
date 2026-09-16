//! One volume a provider offers, and everything done to it in place.

use std::future::Future;

use crate::endpoints::volumes::edit::server::response::Edit;
use crate::endpoints::volumes::stat::server::response::Stat;

/// One named directory of one caller, found by
/// [`VolumeMountManager::get`](super::volume_mount_manager::VolumeMountManager::get).
///
/// What a [`VolumeMountManager`](super::volume_mount_manager::VolumeMountManager)
/// hands back for a name it holds. The verbs that act on an existing
/// volume in place live here — [`stat`](Self::stat) and
/// [`edit`](Self::edit) — and so does the one fact this crate keeps
/// about a volume for itself: whether something is using it, which
/// is the [`lock`](Self::lock).
///
/// # The lock is the mount
///
/// A volume is mounted in at most one container of its caller at a
/// time, and nothing examines or resizes a volume while a container
/// has it. Both rules are one lock, held by whoever is using the
/// volume:
///
/// - A run takes the lock on every volume its request names before
///   it fetches or deploys anything, and holds every one until the
///   run ends — a stop, the container's own end, the caller going
///   away. Every ending unlocks. A lock that is held when a run asks
///   is the run refused,
///   [`VolumeMounted`](crate::shared::containers::response::VolumeMounted).
/// - A [`stat`](crate::endpoints::volumes::stat) takes the lock for
///   the length of the examination, an
///   [`edit`](crate::endpoints::volumes::edit) for the length of the
///   resize, and a [`delete`](crate::endpoints::volumes::delete)
///   takes it and never gives it back, the volume being gone. A lock
///   that is held when any of them asks is that request refused: a
///   delete with its own
///   [`Mounted`](crate::endpoints::volumes::delete::server::response::Frame::Mounted),
///   a stat or an edit with the endpoint's error.
///
/// There is no verb on a volume that does not take the lock. A volume
/// is not watched on its own: the filetree of a container it is
/// mounted in is where it is seen changing, and that tree includes
/// every volume mount.
///
/// The handlers do all of that. A provider's [`lock`](Self::lock) is
/// a try-lock and nothing more: it takes the lock or says it is held,
/// and it never waits, because the party that holds it may hold it
/// for the life of a container and a request cannot queue behind
/// that. The three lock methods are synchronous and cannot fail: a
/// lock is a flag the provider keeps in memory, read and written in
/// one step, and a provider that kept it anywhere else would be
/// making a request wait on a store to learn whether it may run.
/// What a held lock MEANS — refused, mounted, an error — is
/// decided by the handlers, never by the provider, which is why
/// [`VolumeMountManager::delete`](super::volume_mount_manager::VolumeMountManager::delete)
/// answers nothing about mounts.
///
/// # A volume is a handle, not a snapshot
///
/// Whatever the provider holds a volume by: an entry in its own
/// cache, a path, a row. Nothing is read at
/// [`get`](super::volume_mount_manager::VolumeMountManager::get) time — every
/// method asks the volume as it is now — and a handle may outlive
/// the volume, since a delete on the manager takes a name and not a
/// handle. A method on a handle to a deleted volume fails with the
/// provider's error, which is the same answer a stale name gets.
pub trait Volume: Send + Sync {
    /// Whatever this provider's volumes fail with. The same type its
    /// [`VolumeMountManager`](super::volume_mount_manager::VolumeMountManager) fails
    /// with, which that trait's bound states; see
    /// [`VolumeMountManager::Error`](super::volume_mount_manager::VolumeMountManager::Error)
    /// for why it is the provider's own.
    type Error: Send + 'static;

    /// Take the lock: `true` is taken, and the caller holds it until
    /// its [`unlock`](Self::unlock); `false` is held by another, and
    /// nothing changed.
    ///
    /// # It never waits
    ///
    /// The holder may be a container that runs for hours, and a
    /// request that queued behind it would be a request that never
    /// answered. So this answers at once — it is not even `async` —
    /// and what a `false` means is the handler's to decide; see the
    /// trait.
    fn lock(&self) -> bool;

    /// Give the lock back: `true` is the lock released, and it was
    /// held; `false` is a lock that was not held, and nothing
    /// changed. Called only by the holder, once per
    /// [`lock`](Self::lock) that answered `true`, and the handlers
    /// keep that count, so a `false` here is a handler's mistake and
    /// not a state a provider has to defend against.
    fn unlock(&self) -> bool;

    /// Whether the lock is held, as of now.
    ///
    /// A fact about now and nothing more: a `false` here is not a
    /// promise that the [`lock`](Self::lock) after it answers `true`.
    /// No handler asks it; it is for the provider's own use — a
    /// listing that wants to say which volumes are in use, a log.
    fn locked(&self) -> bool;

    /// The volume, examined: how much of it is used and what is in
    /// it.
    ///
    /// The two fields a listing does not carry, because each is a
    /// walk of the volume — a size to sum, a tree to hash — and a
    /// listing that paid for every volume's walk would pay for the
    /// ones nobody asked about.
    ///
    /// Called under the lock, so no container writes while it walks,
    /// and what it reports is the volume at rest.
    fn stat(&self) -> impl Future<Output = Result<Stat, Self::Error>> + Send;

    /// Change how big the volume may be, in BYTES.
    ///
    /// Capacity and nothing else. Called under the lock, so no
    /// container writes while it resizes.
    ///
    /// # Why a rename is not an edit
    ///
    /// Because [`name`](crate::endpoints::volumes::list::server::response::Volume::name)
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
    /// [`bytes_used`](crate::endpoints::volumes::stat::server::response::Stat::bytes_used)
    /// — a volume holding more than it would then reserve — is
    /// [`Edit::ContentTooLarge`]. Neither is an error, and in both the
    /// size is as it was. The provider is what knows its room and the
    /// volume's contents, which is why both answers are the provider's
    /// to give.
    fn edit(&self, bytes: u64) -> impl Future<Output = Result<Edit, Self::Error>> + Send;
}
