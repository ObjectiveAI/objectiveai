//! A container that is running, and what can be done with one.

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::Stream;

use crate::shared::error::Error;

/// A running container, as far as this crate needs one.
///
/// What a
/// [`ContainerDeployer`](super::container_deployer::ContainerDeployer)
/// hands back. The type itself is the provider's — a process handle, a
/// name, an instance id, a struct with all three — and this is the part
/// of it this crate has to be able to reach.
///
/// # Why these three and not the others
///
/// A container is asked for more than this. An MCP exchange, a
/// filetree, a file moved to another container — all of them arrive on
/// channels inside a scope, and none of them is here.
///
/// The line is whether a provider can do it from OUTSIDE. An MCP
/// exchange is HTTP against a port, and a provider that can reach the
/// port relays it without the container's cooperation. Reading and
/// writing a file are not like that: the filesystem is the container's,
/// and only whatever deployed it can reach in. Stopping is the same —
/// only the thing that started it knows how.
///
/// So what is here is what nothing else can do, and the rest is
/// relaying. A filetree will join this list when it is written, because
/// watching a filesystem is reaching into one; a transfer probably
/// will too, since it is a read and a write with nothing in between.
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
    /// One type for all three methods, because a provider that told
    /// them apart would be doing it for its own benefit rather than
    /// this crate's. Nothing here branches on which operation failed;
    /// what it does with one is put it on the wire.
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

    /// Stop it.
    ///
    /// Returns when the container is stopped, the way a deploy returns
    /// when it is running. Not when a stop has been requested, not when
    /// a signal has been sent — an implementation that waits for
    /// something waits before it returns [`Ok`].
    ///
    /// # A container that is already gone is a stop that worked
    ///
    /// Because what a caller wanted is the state, not the act. A
    /// container that exited on its own, or crashed, or was stopped by
    /// something else, is a container that is not running — which is
    /// the whole of what was asked for.
    ///
    /// The alternative is an error every caller has to recognise and
    /// then ignore, which is a way of writing the same rule in every
    /// caller instead of once here.
    ///
    /// # What it does to the scope is not this
    ///
    /// A scope ends when its provider finishes it, and stopping a
    /// container is one of the things that leads to that. This does not
    /// send a frame and does not know there is one to send.
    fn stop(&self) -> impl Future<Output = Result<(), Self::Error>> + Send;

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
    /// # The content's errors are the CALLER's
    ///
    /// Which is the one asymmetry here.
    /// [`Self::Error`](Self::Error) is the provider's, and it is what
    /// this returns — but the [`Error`] inside the stream came off the
    /// wire, from whoever is supplying the bytes.
    ///
    /// So an [`Err`] item is not a failure of the container. It is the
    /// far end saying its content stopped, which the endpoint's frame
    /// calls "the full content was not streamed". An implementation
    /// that sees one abandons the write and reports whatever it makes
    /// of that — there is nothing partial at the path either way.
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
        content: Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send>>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
