//! A container that is running, and what can be done with one.

use std::fmt;
use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::{Sink, Stream};

use crate::shared::error::Error;

/// A running container, as far as this crate needs one.
///
/// What a
/// [`ContainerDeployer`](super::container_deployer::ContainerDeployer)
/// hands back. The type itself is the provider's — a process handle, a
/// name, an instance id, a struct with all three — and this is the part
/// of it this crate has to be able to reach.
///
/// # What belongs here
///
/// Whatever needs something only the thing that deployed the container
/// has. Not "what a provider cannot do from outside" — that was the
/// first answer and [`connect`](Self::connect) is what disproves it. An
/// MCP exchange IS made from outside, over a socket, and it still
/// belongs here: reaching the port means knowing which host-side
/// address that container's port ended up at, and only the deploy
/// learned it.
///
/// So the four are the four facts a provider keeps to itself. Where the
/// container's filesystem is, for [`read`](Self::read) and
/// [`write`](Self::write). How to end it, for [`stop`](Self::stop).
/// Where to reach it, for [`connect`](Self::connect).
///
/// A filetree will join them when it is written, since watching a
/// filesystem is knowing where one is. A transfer will not — it is a
/// read on one container and a write on another, both already here.
///
/// # `Send` and `Sync`
///
/// Because a scope is served by more than one task — a dispatcher
/// reading channel requests, a pump answering one — and a container is
/// reached from any of them, behind a shared reference.
///
/// Which is also why every method takes `&self`. Nothing here consumes
/// a container, including [`stop`](Self::stop): it is stopped while
/// whatever is holding it still holds it, and dropping it afterwards is
/// a separate act that this crate does not define.
pub trait Container: Send + Sync {
    /// Why something a container was asked for did not happen.
    ///
    /// The provider's own, for the reason
    /// [`ContainerDeployer::Error`](super::container_deployer::ContainerDeployer::Error)
    /// is: a file that would not open, a runtime that would not stop, a
    /// disk that filled — these belong to a runtime and a kernel, and
    /// this crate names neither.
    ///
    /// One type for both methods that can fail, because a provider
    /// that told them apart would be doing it for its own benefit
    /// rather than this crate's. Nothing here branches on which
    /// operation failed; what it does with one is put it on the wire.
    ///
    /// [`stop`](Self::stop) is not one of them — see it for why.
    ///
    /// It need not be the same type a deploy fails with. A provider
    /// whose deploy and whose reads go wrong in the same ways uses one
    /// for both, and one whose reads can only fail in ways a deploy
    /// never could says so.
    ///
    /// [`Send`] and `'static` for the same reasons every other error
    /// here is: the future carrying it is [`Send`], so its output has
    /// to be, and it outlives the call that produced it.
    type Error: Send + 'static;

    /// The half of a connection that bytes arrive on.
    ///
    /// Whatever comes off the socket, in whatever pieces it comes off
    /// in. Those pieces mean nothing — a protocol riding this
    /// reassembles, exactly as it would from a socket, because that is
    /// what this is.
    ///
    /// # [`Unpin`], which is the bound worth arguing about
    ///
    /// [`StreamExt::next`](futures_util::StreamExt::next) requires it.
    /// Without it every place that reads one does the manual
    /// [`poll_next`](Stream::poll_next) dance or boxes it first.
    ///
    /// An implementation holding something that is not [`Unpin`] boxes
    /// it once, where it is built. A consumer without the bound pays at
    /// every call. That asymmetry is the whole argument.
    ///
    /// [`Send`] and `'static` because it outlives the call that made it
    /// and is polled from wherever the reading happens, which is not
    /// where it was built.
    type Reader: Stream<Item = Result<Bytes, Self::Error>>
        + Send
        + Unpin
        + 'static;

    /// The half of a connection that bytes go out on.
    ///
    /// Its [`Error`](Sink::Error) is [`Self::Error`], so a write
    /// failing and a read failing are the same kind of thing — which
    /// they are, being two directions of one socket.
    ///
    /// # Closing it does not close the [`Reader`](Self::Reader)
    ///
    /// [`close`](futures_util::SinkExt::close) is a half-close, and
    /// both protocols that will ride this depend on it being one.
    /// pgwire's `Terminate` is a client saying it is done talking while
    /// the server is still answering; an HTTP request whose response is
    /// still coming is the same shape.
    ///
    /// An implementation that tore down the read half here would break
    /// both, and would do it in a way that looks like the far end
    /// hanging up.
    ///
    /// # [`Unpin`], for the same reason
    ///
    /// [`SinkExt::send`](futures_util::SinkExt::send) requires it, and
    /// without it a writer is `poll_ready`, `start_send`, `poll_flush`
    /// by hand at every site.
    type Writer: Sink<Bytes, Error = Self::Error> + Send + Unpin + 'static;

    /// Stop it.
    ///
    /// Returns when the container is stopped, the way a deploy returns
    /// when it is running. Not when a stop has been requested, not when
    /// a signal has been sent — an implementation that waits for
    /// something waits before it resolves.
    ///
    /// # It cannot fail, and the reason is not optimism
    ///
    /// Two things would have gone in a [`Result`] and neither belongs
    /// there.
    ///
    /// A container that is ALREADY GONE is not a failure. It exited on
    /// its own, or crashed, or something else stopped it — and what was
    /// asked for is the state rather than the act, so a container that
    /// is not running is the whole of it. An error there would be one
    /// every caller has to recognise and then ignore, which is writing
    /// the same rule in every caller instead of once here.
    ///
    /// A container that WILL NOT stop is a failure, and there is
    /// nothing to do about it. No frame means "the container would not
    /// stop": a scope ends when its provider finishes it, and it
    /// finishes either way. The error would have nowhere to go and no
    /// caller able to act on it.
    ///
    /// What an implementation does about the second is its own business
    /// — retry it, log it, leave it for whatever reaps stragglers. That
    /// is operational, and this protocol has no opinion.
    ///
    /// Which leaves one thing this future means: as far as the provider
    /// is concerned, that container is done.
    ///
    /// # What it does to the scope is not this
    ///
    /// A scope ends when its provider finishes it, and stopping a
    /// container is one of the things that leads to that. This does not
    /// send a frame and does not know there is one to send.
    fn stop(&self) -> impl Future<Output = ()> + Send;

    /// Open a socket to a port inside it.
    ///
    /// A byte pipe and nothing more. What rides it is the caller's
    /// business — MCP over HTTP, pgwire, whatever a provider arranged
    /// for itself — and this neither knows nor frames any of it.
    ///
    /// # Why bytes rather than the protocol
    ///
    /// Because there is more than one protocol and there always was. An
    /// MCP server speaks HTTP, a database conduit speaks pgwire, and
    /// something a provider set up for its own purposes speaks whatever
    /// it likes. A method shaped like one of them would serve that one
    /// and be worked around by the rest.
    ///
    /// It is also what the container actually offers. A port is a byte
    /// stream; anything more structured is something built on top, by
    /// whoever knows which protocol this is.
    ///
    /// # It dials once
    ///
    /// There is no waiting here and no retry. A
    /// [`ContainerDeployer`](super::container_deployer::ContainerDeployer)
    /// hands back a container whose declared ports already accept, so
    /// there is nothing to wait for — see the section there for why
    /// that waiting belongs to a provider and not to this crate.
    ///
    /// A refusal therefore means something is wrong rather than that
    /// something is slow, which is what makes it worth reporting.
    ///
    /// # The port has to have been declared
    ///
    /// In [`Deployment::ports`](super::deployment::Deployment::ports),
    /// before the container was deployed. Connecting to one that was
    /// not is not required to work, and on a runtime that publishes
    /// ports when it creates a container it cannot — there is nothing
    /// left to publish through.
    ///
    /// This is the other half of the rule that field states. A provider
    /// undertook to make those ports reachable; it undertook nothing
    /// about any others.
    ///
    /// # Two halves, handed over separately
    ///
    /// Because they are pumped by different tasks — one reading the
    /// socket and writing frames, one reading frames and writing the
    /// socket — and a single duplex value would have to be taken apart
    /// before either could start.
    ///
    /// # Failing to connect is not an empty stream
    ///
    /// It is an [`Err`] here. Nothing listening on that port is the
    /// ordinary case rather than an exceptional one —
    /// [`mcp_plugin`](crate::endpoints::mcp_plugin::run::client::request::Frame::mcp_port)
    /// documents a wrong port as exactly that — and folding it into the
    /// [`Reader`](Self::Reader) would make "could not connect"
    /// indistinguishable from "connected, then closed".
    ///
    /// Which is the confusion this protocol works hardest to prevent
    /// everywhere else, and there is no reason to introduce it here.
    ///
    /// # It says nothing about what answers
    ///
    /// A connection is a connection. Whether the thing on the other end
    /// is the server that was expected, or is ready, or will answer at
    /// all, is found out by talking to it — the same distinction a
    /// [`ContainerDeployer`](super::container_deployer::ContainerDeployer)
    /// draws when it says a container running is not a server bound.
    fn connect(
        &self,
        port: u16,
    ) -> impl Future<
        Output = Result<(Self::Reader, Self::Writer), Self::Error>,
    > + Send;

    /// Read one file out of it.
    ///
    /// The future resolves when the read has STARTED, not when it has
    /// finished — what it resolves to is the file, and that is where
    /// the reading shows up. A file that cannot be opened says so as
    /// the stream's first item rather than as a failure here, which is
    /// what lets both endings arrive by one route.
    ///
    /// # One file, never a directory
    ///
    /// See [`read`](crate::shared::container::read) for why. A path
    /// that names a directory is a read that fails, not a read that
    /// produces something else.
    ///
    /// # The pieces are the implementation's to choose
    ///
    /// One item or a thousand, and how large each is, carries no
    /// meaning — the far end concatenates. What it should not do is
    /// hold the whole file to send it as one piece: a caller reading a
    /// large file gets it as it comes, and buffering here would undo
    /// that for every reader.
    ///
    /// # An [`Err`] ends it
    ///
    /// Whatever arrived before it is a prefix of the file, and nothing
    /// says how much is missing. A reader cannot tell a refused read
    /// from a truncated one, which is what the endpoint's own frame
    /// says too: "the file was not read, or not all of it".
    ///
    /// # The path
    ///
    /// Components from the container's root, the same frame of
    /// reference a [`filetree`](crate::shared::filetree) stream uses.
    /// Components rather than a joined string, because joining invents
    /// a separator that then has to be escaped out of names containing
    /// it.
    fn read(
        &self,
        path: &[String],
    ) -> impl Future<
        Output = Pin<
            Box<dyn Stream<Item = Result<Bytes, Self::Error>> + Send>,
        >,
    > + Send;

    /// Write one file into it.
    ///
    /// Resolves when the file is at the path, or when it is not. Unlike
    /// [`read`](Self::read), this does not resolve early: there is one
    /// answer and it is not known until the content has been consumed.
    ///
    /// # A write replaces, and leaves nothing partial
    ///
    /// Whatever is at the path is replaced, whole. An implementation
    /// writes to a temporary and renames it into place, so the path
    /// holds the old file, then nothing, then the new one — never a
    /// prefix of the new one. That holds whether this returns [`Ok`] or
    /// not, and callers are told it does.
    ///
    /// # The content can come from two places
    ///
    /// Which is why its errors are a [`ContentError`] rather than one
    /// type. A caller's content arrives over the wire and fails in the
    /// caller's vocabulary; a
    /// [`transfer`](crate::shared::container::transfer)'s comes from a
    /// [`read`](Self::read) on another container in this same process
    /// and fails in the provider's. See [`ContentError`] for why
    /// neither collapses into the other.
    ///
    /// Either way an [`Err`] item is not a failure of the container
    /// being written into. It is the source saying it has no more to
    /// give — what the endpoint's frame calls "the full content was not
    /// streamed" — and an implementation that sees one abandons the
    /// write. There is nothing partial at the path either way.
    ///
    /// # What it RETURNS is still the provider's
    ///
    /// [`Self::Error`](Self::Error), whatever ended the content. A
    /// write abandoned because its source stopped is still a write that
    /// did not land, and this reports what the container made of that
    /// rather than repeating why the bytes ran out — which the thing
    /// supplying them already knows.
    ///
    /// # Boxed rather than generic
    ///
    /// The stream is built by whatever is relaying the caller's
    /// content, and there is one shape of it. A type parameter would
    /// let an implementation be generic over something that never
    /// varies, at the cost of a bound on every signature that mentions
    /// one.
    ///
    /// [`Send`] and `'static` because it is polled wherever the write
    /// happens. Not [`Sync`] — one task owns it and polls it through
    /// `&mut`, so a shared reference to it never exists.
    fn write(
        &self,
        path: &[String],
        content: Pin<
            Box<
                dyn Stream<Item = Result<Bytes, ContentError<Self::Error>>>
                    + Send,
            >,
        >,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// The content for a write ran out early, and this is whose fault it
/// was.
///
/// What an item of the stream
/// [`Container::write`] consumes
/// fails with. It is an enum because a write's content has two origins
/// and they fail in incompatible vocabularies.
///
/// # The two origins
///
/// A caller's write sends its content over the wire, as responses on a
/// channel the provider opened. When that stops early the frame says so
/// with a [`shared::error::Error`](crate::shared::error::Error) — one
/// JSON value, from somebody else's process, meaning whatever that
/// caller meant. That is [`Wire`](Self::Wire).
///
/// A [`transfer`](crate::shared::container::transfer) has no wire in
/// it. Both containers are the provider's, so the bytes go from a
/// [`read`](Container::read) on one straight into a
/// write on the other without ever becoming frames — which is the whole
/// point of having a transfer rather than a read piped through a
/// caller. A read fails with the provider's own error, and that is
/// [`Container`](Self::Container).
///
/// # Why not flatten them
///
/// Making everything a
/// [`shared::error::Error`](crate::shared::error::Error) would mean a
/// provider converting its own read failure into opaque JSON so it
/// could hand it to its own write, in the same process, for nobody's
/// benefit. That is the mistake
/// [`ContainerDeployer::Error`](super::container_deployer::ContainerDeployer::Error)
/// exists to avoid: the type that knows most about what happened, lost
/// at the one moment it is in hand.
///
/// Making everything the provider's own error is not available. A
/// caller's failure is a JSON value from another process, and nothing
/// turns one of those into a runtime's error type.
///
/// So neither collapses into the other, and the type says so.
///
/// # It is generic rather than tied to the trait
///
/// [`Container::write`] uses it as
/// `ContentError<Self::Error>`, but nothing here names that. It is a
/// plain two-armed enum over "the wire" and "something else", which is
/// what lets a provider hold one before it has decided which container
/// it is about to write into.
#[derive(Debug, Clone, PartialEq)]
pub enum ContentError<E> {
    /// A caller's content stopped.
    ///
    /// Relayed verbatim from the channel it arrived on, meaning
    /// whatever the caller meant by it. A provider does not read it and
    /// could not usefully — see
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    ///
    /// It is not a failure of the container being written into. The
    /// write is abandoned because there is nothing left to write, not
    /// because anything here went wrong.
    Wire(Error),
    /// Another container's read stopped.
    ///
    /// The provider's own error, from the
    /// [`read`](Container::read) feeding this write.
    /// Which happens for a
    /// [`transfer`](crate::shared::container::transfer), where the
    /// source is a container rather than a caller.
    ///
    /// Whether it is a refused read or a truncated one is not
    /// something a read distinguishes, so this does not either.
    Container(E),
}

impl<E> fmt::Display for ContentError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentError::Wire(_) => {
                f.write_str("the caller's content stopped")
            }
            ContentError::Container(error) => {
                write!(f, "the source container's read stopped: {error}")
            }
        }
    }
}

impl<E> std::error::Error for ContentError<E>
where
    E: std::error::Error + 'static,
{
    /// [`Wire`](ContentError::Wire) has no source, because what it
    /// carries is not a Rust error and deliberately does not implement
    /// one — see
    /// [`shared::error::Error`](crate::shared::error::Error). A caller
    /// that wants what is inside it matches the variant.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ContentError::Container(error) => Some(error),
            ContentError::Wire(_) => None,
        }
    }
}
