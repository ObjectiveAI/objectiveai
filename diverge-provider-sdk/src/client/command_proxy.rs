//! Running the commands a plugin cannot run itself.

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::Stream;

/// What runs a Diverge command on a plugin's behalf.
///
/// A plugin has no CLI binary in its container and no daemon it is
/// allowed to dial, so a command it wants run has to be run by somebody
/// who can. That is the caller. The plugin asks, the provider relays,
/// and this is what executes it.
///
/// # One ask, then a stream
///
/// A [`Command`](crate::endpoints::mcp_plugin::run::server::channel_request::Frame::Command)
/// opens the channel and nothing follows it in that direction, so this
/// is called once per channel and takes the whole request by value.
/// What comes back is as many items as the command produces, then a
/// finish — a command yielding a thousand rows delivers them as they
/// come rather than as one document assembled first.
///
/// Which is why there is no choice of shape here as there is on
/// [`OciProxy`](super::oci_proxy::OciProxy). A command is always a
/// stream, even when it yields one item, and there is no head because
/// there is no HTTP to carry one.
///
/// # The rule it relies on is the specification's, not the frame's
///
/// Nothing in
/// [`ServerFrame`](crate::frame::server::ServerFrame) stops a second
/// channel request arriving on a channel that already has one. This
/// signature says there is only ever one because the endpoint says so,
/// which means a dispatcher needs a stated policy for a frame that
/// should not exist rather than a place to put it.
///
/// # Opaque, and for a different reason than Postgres
///
/// [`PostgresProxy`](super::postgres_proxy::PostgresProxy) is opaque
/// because parsing pgwire would mean keeping up with pgwire. This is
/// opaque because the command vocabulary is not this specification's to
/// define — it belongs to the CLI, which gains subcommands on its own
/// schedule, and a protocol that named them would be revised every time
/// one appeared.
///
/// So a provider relays and never reads, and cannot tell one command
/// from another. Which also means it cannot decide it disapproves of
/// one: what a plugin may ask for is settled between the plugin and the
/// caller, using the
/// [`identity`](crate::endpoints::mcp_plugin::run::client::request::Frame::identity)
/// the caller supplied. An implementation that wants to refuse
/// something refuses it here, and that decision is a caller's.
///
/// # Failure is an item, not a [`Result`]
///
/// There is no error variant on this channel, and the items are
/// [`Bytes`] rather than a [`Result`], because a command that fails
/// already knows how to say so: it emits its failure as one of its
/// items, in whatever shape the CLI uses for that. A `Result` item would
/// have nothing to encode into and a dispatcher's only options for an
/// `Err` would be to drop it or to finish — and finishing is what the
/// stream ending already does.
///
/// So a command that fails partway ends the same way any other does. The
/// items that arrived are what it produced, and the finish says there
/// are no more.
///
/// Which is what each of these traits does, into a different vocabulary
/// each time — [`OciProxy`](super::oci_proxy::OciProxy) answers in HTTP
/// statuses, [`PostgresProxy`](super::postgres_proxy::PostgresProxy) in
/// pgwire, and this one in the CLI's own.
pub trait CommandProxy: Send + Sync {
    /// Run one command, and stream back what it produces.
    ///
    /// # The request is owned
    ///
    /// Unlike [`McpProxy::handle`](super::mcp_proxy::McpProxy::handle),
    /// which can borrow because it is answered in the task that received
    /// the frame. A command's answers can outlast that task by as long
    /// as the command runs, so a dispatcher hands the bytes to something
    /// long-lived — and a borrow cannot cross that.
    ///
    /// It costs nothing to say so. Slicing the payload out of the
    /// arriving frame with
    /// [`Bytes::slice_ref`](bytes::Bytes::slice_ref) is a refcount and
    /// not a copy.
    ///
    /// What arrives is the payload the plugin sent, with the frame
    /// header and the variant tag already off it.
    ///
    /// # The future is [`Send`]
    ///
    /// Because a plugin may have several commands in flight and a
    /// provider serves several plugins, so these overlap by design. It
    /// is spelled out rather than left to `async fn`, which promises
    /// nothing about the future it returns.
    ///
    /// Note that the future resolves when the command has STARTED, not
    /// when it has finished — what it resolves to is the stream of the
    /// command's items, and that is where the running shows up.
    ///
    /// # The bounds on the stream
    ///
    /// [`Send`] and `'static` because it is polled from wherever the
    /// answer is written, which is not where it was built.
    ///
    /// [`Sync`] is NOT required, and used to be. Whoever writes the
    /// answer owns this and polls it through `&mut`, so a shared
    /// reference to it never exists — and requiring one turned away the
    /// obvious way to write a stream, since an `async_stream` generator
    /// is [`Sync`] only if everything it awaits is.
    ///
    /// Written out rather than aliased, as in the sibling proxies that
    /// return one. An alias would hide exactly those bounds from the
    /// signature a reader is checking theirs against.
    fn run(
        &self,
        request: Bytes,
    ) -> impl Future<
        Output = Pin<Box<dyn Stream<Item = Bytes> + Send + 'static>>,
    > + Send;
}
