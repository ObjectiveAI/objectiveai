//! The directories a provider offers: the namespace, and the verbs
//! that make and unmake its names.

use std::future::Future;

use super::volume;
use crate::endpoints::volumes::create::server::response::Creation;
use crate::endpoints::volumes::list::server::response::Volume;

/// A namespace of named directories, one per caller.
///
/// The seven [`volumes`](crate::endpoints::volumes) endpoints are
/// seven verbs over one thing, and this is the thing: what a listing
/// reports, what a create adds and a delete takes away, and where a
/// name is looked up. The verbs on a volume that exists —
/// examining it, resizing it — are on the
/// [`Volume`](volume::Volume) that [`get`](Self::get) hands back; the
/// verbs on the NAMESPACE are here. A provider that implements both
/// can answer all seven; there is nothing else they need.
///
/// # Why the split falls where it does
///
/// A name is made and unmade here because only the namespace can
/// answer whether it is free, and only the namespace can strike it.
/// Capacity is here for the same reason: how large a volume may be
/// made, and how far one may grow, are facts about the provider's
/// room, not about any one volume. A volume answers for itself only
/// where it is being acted on in place, and the lock every such act
/// takes first is on the volume — see [`Volume`](volume::Volume) for
/// the lock, which is the whole of how this crate keeps a mounted
/// volume from being examined, resized, deleted, or mounted twice.
/// Nothing here is asked about mounts.
///
/// # Every method takes a `client_identity`
///
/// Because a volume's name is unique within the caller it was listed
/// to, not globally — the fact
/// [`Mount::client_identity`](super::mount::Mount::client_identity)
/// exists to record. Two callers each holding a volume called `work` is
/// ordinary, so a method taking only a name would be asking a question
/// with more than one answer.
///
/// It is an argument rather than something the manager was built with,
/// so that ONE manager serves every caller on every connection. A
/// manager per caller would be a manager per connection, built and
/// dropped around a namespace that outlives both.
///
/// Where a provider gets the identity is its own business — it is
/// whatever authenticated the connection, and this crate never mints
/// one, parses one, or compares two.
///
/// # It does not know that containers exist
///
/// Deliberately, and the absence is not an oversight. A
/// [`Mount`](super::mount::Mount) reaches a
/// [`ContainerDeployer`](super::container_deployer::ContainerDeployer)
/// as a name and an identity, and turning that into a directory is the
/// deployer's business — the same way that publishing a port is the
/// deployer's business rather than something described here.
///
/// So there is no `resolve` on this trait, and the two do not depend on
/// each other. A provider implements both and knows how its own
/// volumes are laid out; a crate that put a path between them would be
/// inventing a representation for a directory that neither trait needs
/// to agree on. The run handler does ask this trait for each volume
/// a request names, to lock it — but it asks for the handle, never
/// for where it is.
///
/// # What is deliberately not decided
///
/// What earns a failure. The wire has one
/// [`Error`](crate::shared::error::Error) per endpoint and says nothing
/// about when it is sent, so whether a
/// [`create`](Self::create) over an existing name fails is the
/// provider's to answer. This trait gives it somewhere to say no and
/// does not say when.
///
/// The cases the wire does decide each have an answer of their own
/// rather than a failure: a [`create`](Self::create) or an
/// [`edit`](volume::Volume::edit) to a size the provider cannot
/// reserve is insufficient capacity; an edit below
/// [`bytes_used`](crate::endpoints::volumes::stat::server::response::Stat::bytes_used)
/// is content too large; a delete of a mounted volume is refused as
/// mounted — and that last one the handler answers from the lock,
/// before this trait is asked.
pub trait VolumeManager: Send + Sync {
    /// Whatever this provider's volumes fail with.
    ///
    /// Its own type, not
    /// [`shared::error::Error`](crate::shared::error::Error). A
    /// provider has failures of its own shape — a quota, a filesystem,
    /// a remote store — and flattening one into the protocol's error is
    /// the handler's job, at the point where a frame is written.
    ///
    /// The same choice
    /// [`ContainerDeployer::Error`](super::container_deployer::ContainerDeployer::Error)
    /// makes, for the same reason: a crate that named a provider's
    /// error type would be describing something it cannot see.
    ///
    /// `Send + 'static` because it crosses tasks and outlives the call
    /// that produced it.
    type Error: Send + 'static;

    /// What [`get`](Self::get) hands back: the provider's own handle to
    /// one volume, failing with the same error this trait fails with,
    /// so a handler that holds both has one error to flatten.
    type Volume: volume::Volume<Error = Self::Error>;

    /// Which volumes this caller has.
    ///
    /// Everything the caller may name, which is the whole of what a
    /// listing is for: a caller cannot ask about a volume it has not
    /// been told about, so this is what makes any of the others
    /// possible.
    ///
    /// Empty is a caller with no volumes, which is where every caller
    /// starts. It is not a failure.
    ///
    /// # Order is the provider's
    ///
    /// Nothing here sorts, and the endpoint does not either. A provider
    /// that returns them in a stable order gives a caller a stable
    /// listing for free; one that does not has broken nothing, because
    /// a [`name`](crate::endpoints::volumes::list::server::response::Volume::name)
    /// is the handle and a position is not.
    fn list(
        &self,
        client_identity: &str,
    ) -> impl Future<Output = Result<Vec<Volume>, Self::Error>> + Send;

    /// The caller's volume under `name`, or [`None`] where the caller
    /// has none by it.
    ///
    /// The lookup every verb on an existing volume starts with: a
    /// stat, an edit, a delete, and a run that names the volume in a
    /// mount all ask this first and then act on what comes
    /// back. What comes back is the provider's own handle — see
    /// [`Volume`](volume::Volume) — and asking for it changes
    /// nothing.
    ///
    /// # `None` is an answer
    ///
    /// A name that is not in this caller's listing — never there,
    /// deleted, or another caller's — is [`None`], not a failure, and
    /// the handler turns it into the endpoint's error itself. [`Err`]
    /// is the provider unable to look: a store that did not answer.
    fn get(
        &self,
        client_identity: &str,
        name: &str,
    ) -> impl Future<Output = Result<Option<Self::Volume>, Self::Error>> + Send;

    /// The largest size, in BYTES, a new volume of this caller could
    /// have right now.
    ///
    /// The largest single volume, not the sum of the provider's room:
    /// a volume lives in one place, and the largest place is the
    /// bound a [`create`](Self::create) is held to. A quota on this
    /// caller is the provider's to fold in.
    ///
    /// # It reserves nothing
    ///
    /// A fact about now. A [`create`](Self::create) that arrives after
    /// the room went elsewhere is answered
    /// [`Creation::InsufficientCapacity`] on its own terms, and this
    /// number does not bind it.
    fn create_capacity(
        &self,
        client_identity: &str,
    ) -> impl Future<Output = Result<u64, Self::Error>> + Send;

    /// Make a new volume for this caller, of this size in BYTES.
    ///
    /// The name is the caller's to choose, and it is chosen HERE — this
    /// is the one place a name enters the namespace, and everywhere
    /// else a name is looked up rather than invented.
    ///
    /// # Nothing comes back
    ///
    /// The endpoint answers
    /// [`Created`](crate::endpoints::volumes::create::server::response::Frame::Created)
    /// and nothing else, so a
    /// [`Volume`] handed back here would be a value with nowhere to go.
    /// A caller that wants the created volume described asks for a
    /// [`list`](Self::list), which is the same round trip it would have
    /// spent anyway.
    ///
    /// # Capacity is an answer
    ///
    /// A size the provider cannot reserve is
    /// [`Creation::InsufficientCapacity`], not an error, and nothing
    /// exists as a result. The provider is what knows its own room,
    /// which is why the answer is the provider's to give.
    ///
    /// # A name that is taken
    ///
    /// Is a failure or is not, and this trait does not say which. See
    /// the note on the trait itself: the wire has one error and no
    /// vocabulary for distinguishing the reasons.
    fn create(
        &self,
        client_identity: &str,
        name: &str,
        bytes: u64,
    ) -> impl Future<Output = Result<Creation, Self::Error>> + Send;

    /// How many BYTES this caller's volume could grow by right now.
    ///
    /// Headroom, not a size: what an [`edit`](volume::Volume::edit)
    /// could add to the volume's current
    /// [`bytes`](crate::endpoints::volumes::list::server::response::Volume::bytes)
    /// without being answered
    /// [`Edit::InsufficientCapacity`](crate::endpoints::volumes::edit::server::response::Edit::InsufficientCapacity),
    /// as of now. A name the caller cannot see is a failure. Here and
    /// not on the volume because the answer is about the provider's
    /// room, and a question reserves nothing — so it takes no lock,
    /// and is answered while a container has the volume.
    ///
    /// # It reserves nothing
    ///
    /// A fact about now, on the same terms as
    /// [`create_capacity`](Self::create_capacity): an edit that
    /// arrives after the room went elsewhere is answered on its own
    /// terms.
    fn edit_capacity(
        &self,
        client_identity: &str,
        name: &str,
    ) -> impl Future<Output = Result<u64, Self::Error>> + Send;

    /// Remove the caller's volume under `name`, and everything in it.
    ///
    /// The name leaves the namespace, and a
    /// [`create`](Self::create) may use it again afterwards. [`Ok`] is
    /// the volume gone; [`Err`] is the volume as it was.
    ///
    /// # It is called under the lock, and answers nothing about mounts
    ///
    /// The handler [`lock`](volume::Volume::lock)s the volume first
    /// and answers
    /// [`Mounted`](crate::endpoints::volumes::delete::server::response::Frame::Mounted)
    /// itself when the lock is held — a run has it, or a stat or an
    /// edit is in flight — so by the time this is called nothing is
    /// using the volume, and no handle to it will be asked anything
    /// again. The lock is not given back on success: the volume it
    /// was on is gone, and a provider that keeps a lock somewhere it
    /// must clean up cleans it up here. On failure the handler
    /// unlocks, and the volume is as it was.
    fn delete(
        &self,
        client_identity: &str,
        name: &str,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
