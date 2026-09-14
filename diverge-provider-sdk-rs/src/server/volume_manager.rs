//! The directories a provider offers, and everything done to them.

use std::future::Future;

use futures_util::Stream;

use crate::endpoints::volumes::create::server::response::Creation;
use crate::endpoints::volumes::delete::server::response::Deletion;
use crate::endpoints::volumes::edit::server::response::Edit;
use crate::endpoints::volumes::list::server::response::Volume;
use crate::endpoints::volumes::stat::server::response::Stat;
use crate::shared::filetree;

/// A namespace of named directories, one per caller.
///
/// The eight [`volumes`](crate::endpoints::volumes) endpoints are
/// eight verbs over one thing, and this is the thing. A provider that
/// implements this can answer all of them; there is nothing else they
/// need.
///
/// # Why one trait rather than eight
///
/// Because they share the state, not merely the subject. What
/// [`list`](Self::list) reports is what [`create`](Self::create) added
/// and [`delete`](Self::delete) took away, and what
/// [`watch`](Self::watch) looks inside is one of the same entries.
/// Eight traits would be eight views of one map, with nothing saying
/// they had to be the same map — and a provider free to implement
/// seven of them.
///
/// The endpoints are separate for a different reason: they are separate
/// SCOPES, because a caller asks them one at a time and a watch outlives
/// the others. That is a fact about the wire and it does not reach here.
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
/// to agree on.
///
/// # What is deliberately not decided
///
/// What earns a failure. The wire has one
/// [`Error`](crate::shared::error::Error) per endpoint and says nothing
/// about when it is sent, so whether a
/// [`create`](Self::create) over an existing name fails, and what a
/// [`delete`](Self::delete) does to a volume under a watch, are the
/// provider's to answer. This trait gives each of them somewhere to say
/// no and does not say when.
///
/// The cases the wire does decide each have an answer of their own
/// rather than a failure: a [`create`](Self::create) or an
/// [`edit`](Self::edit) to a size the provider cannot reserve is
/// insufficient capacity; an [`edit`](Self::edit) below
/// [`bytes_used`](crate::endpoints::volumes::stat::server::response::Stat::bytes_used)
/// is content too large; a [`delete`](Self::delete) of a mounted
/// volume is refused as mounted.
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

    /// What [`watch`](Self::watch) hands back: the tree as a stream of
    /// [`filetree`] frames, a snapshot first
    /// and one per change after, ending only at an error or when the
    /// handler drops it. The implementation's own type, so a provider
    /// hands over the stream it has rather than boxing it; the handler
    /// pins it where it reads it.
    type Watch: Stream<Item = Result<filetree::response::Frame, Self::Error>> + Send;

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

    /// One volume, examined: how much of it is used and what is in it.
    ///
    /// The two fields [`list`](Self::list) does not carry, because
    /// each is a walk of the volume — a size to sum, a manifest to
    /// hash — and a listing that paid for every volume's walk would
    /// pay for the ones nobody asked about.
    ///
    /// # A name that is not there is a failure
    ///
    /// A stat has nothing to say about a volume the caller cannot
    /// see, so the error is the answer, and this trait does not say
    /// what the error carries.
    fn stat(
        &self,
        client_identity: &str,
        name: &str,
    ) -> impl Future<Output = Result<Stat, Self::Error>> + Send;

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
    /// Headroom, not a size: what an [`edit`](Self::edit) could add to
    /// the volume's current
    /// [`bytes`](crate::endpoints::volumes::list::server::response::Volume::bytes)
    /// without being answered [`Edit::InsufficientCapacity`], as of
    /// now. A name the caller cannot see is a failure.
    ///
    /// # It reserves nothing
    ///
    /// A fact about now, on the same terms as
    /// [`create_capacity`](Self::create_capacity): an
    /// [`edit`](Self::edit) that arrives after the room went elsewhere
    /// is answered on its own terms.
    fn edit_capacity(
        &self,
        client_identity: &str,
        name: &str,
    ) -> impl Future<Output = Result<u64, Self::Error>> + Send;

    /// Change how big an existing volume may be, in BYTES.
    ///
    /// Capacity and nothing else, which is why this takes the same
    /// three arguments as [`create`](Self::create) and means something
    /// different: there the name is being made, here it is being found.
    ///
    /// # Why a rename is not an edit
    ///
    /// Because [`name`](crate::endpoints::volumes::list::server::response::Volume::name)
    /// is the handle and the only one. Every
    /// [`Mount`](super::mount::Mount) that names a volume names it by
    /// this, and a watch in flight was opened against it — so changing
    /// it would not modify a volume, it would replace one with another
    /// that nothing outstanding can reach.
    ///
    /// A caller that wants a different name makes a volume with it.
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
    fn edit(
        &self,
        client_identity: &str,
        name: &str,
        bytes: u64,
    ) -> impl Future<Output = Result<Edit, Self::Error>> + Send;

    /// Remove a volume and everything in it, unless it is mounted.
    ///
    /// The name leaves the namespace, and a
    /// [`create`](Self::create) may use it again afterwards.
    ///
    /// # A mounted volume is not deleted
    ///
    /// A volume [`mounted`](super::mount::Mount) into a container that
    /// is still running is never deleted. A provider answers
    /// [`Deletion::Mounted`] and changes nothing — the one rule about
    /// a volume in use that the wire fixes, and it has its own answer
    /// rather than an error because a caller acts on it differently:
    /// stop the container, ask again.
    ///
    /// The provider is what knows whether a volume is mounted, which
    /// is why the answer is the provider's to give and not the
    /// handler's to check.
    ///
    /// # A watched volume is the provider's
    ///
    /// A [`watch`](Self::watch) is not a mount. A provider may delete
    /// a volume somebody is still watching and let the watch end, or
    /// refuse with its own error; the wire does not say.
    ///
    /// Said plainly because the alternative is that it gets assumed.
    /// A caller that needs a volume gone AND needs nothing to be
    /// watching it arranges the second itself.
    fn delete(
        &self,
        client_identity: &str,
        name: &str,
    ) -> impl Future<Output = Result<Deletion, Self::Error>> + Send;

    /// Watch a volume's tree, and report what changes in it.
    ///
    /// A snapshot first, then one frame per change, for as long as the
    /// stream is held. See
    /// [`filetree::response::Frame`] for the variants and for what
    /// makes the sequence replay-safe.
    ///
    /// # Dropping the stream is how a watch ends
    ///
    /// There is no `unwatch`. A caller's
    /// [`stop`](crate::endpoints::volumes::watch::client::channel_request::Frame)
    /// ends the scope, the handler drops what it was reading, and a
    /// provider stops walking a tree nobody is listening about. One
    /// less method, and no way for a handler to forget the second half
    /// of a pair.
    ///
    /// # Why the failure is outside the stream and not only inside it
    ///
    /// Because a volume that is not there is knowable before the first
    /// item, and it is a different fact from a watch that started and
    /// then stopped. Collapsing the two would make "there was never
    /// anything to watch" indistinguishable from "what you were
    /// watching went away", which is the distinction this protocol
    /// works hardest to preserve everywhere else.
    ///
    /// It diverges from a read inside a container, which folds its
    /// failure into the stream, and the difference is real: a path
    /// inside somebody else's container may not be checkable without
    /// beginning to read it, and a name in a namespace this trait OWNS
    /// always is.
    ///
    /// Both ends up as the same
    /// [`Error`](crate::endpoints::volumes::watch::server::response::Frame::Error)
    /// frame, because the endpoint has one place to put a failure. That
    /// flattening is the handler's to do, and a trait that had done it
    /// in advance would have thrown away a distinction the handler
    /// might want for something else — a log, a metric, a retry it only
    /// attempts for one of them.
    ///
    /// # An error in the stream ends it
    ///
    /// The watch is over; there is no resuming after one. A provider
    /// that can recover keeps the stream going and says nothing,
    /// because a consumer folding these frames cannot tell a gap from
    /// quiet and must not be handed one.
    fn watch(
        &self,
        client_identity: &str,
        name: &str,
    ) -> impl Future<Output = Result<Self::Watch, Self::Error>> + Send;
}
