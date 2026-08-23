//! Running the commands a plugin cannot run itself.

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::Stream;

use crate::shared::http::{request, response};

/// What runs a Diverge command on a plugin's behalf.
///
/// A plugin has no CLI binary in its container and no daemon it is
/// allowed to dial, so a command it wants run has to be run by somebody
/// who can. That is the caller. The plugin asks, the provider relays,
/// and this is what executes it.
///
/// # One ask, one answer
///
/// A [`Command`](crate::endpoints::mcp_plugin::run::server::channel_request::Frame::Command)
/// opens the channel and nothing follows it in that direction, so this
/// is called once per channel. What comes back is a head and a body,
/// and the channel finishes when the body does.
///
/// # It is an HTTP exchange, and the body is still the CLI's
///
/// The envelope is HTTP — a method, a path, headers, a body, answered
/// with a status, headers and a body — and that is all the envelope
/// is. What a command IS lives in the path, and what it produces lives
/// in the body, and this specification says nothing about either.
///
/// Which is the same line
/// [`OciProxy`](super::oci_proxy::OciProxy) draws in the other
/// direction: a registry request is HTTP because a registry speaks
/// HTTP, and a command is HTTP because it needed an envelope and this
/// endpoint already carries one. Neither makes the contents this
/// protocol's business.
///
/// The vocabulary belongs to the CLI, which gains subcommands on its
/// own schedule, and a protocol that enumerated them would be revised
/// every time one appeared.
///
/// # A provider cannot tell one command from another
///
/// It relays and never reads, which also means it cannot decide it
/// disapproves of one. What a plugin may ask for is settled between the
/// plugin and the caller, using the
/// [`identity`](crate::endpoints::mcp_plugin::run::client::request::Frame::identity)
/// the caller supplied — so an implementation that wants to refuse
/// something refuses it HERE, with a status, and that decision is a
/// caller's.
///
/// # Failure is a status, not an item
///
/// It used to be an item: the stream carried [`Bytes`] rather than a
/// [`Result`], and a command that failed emitted its failure as one of
/// the things it produced, in whatever shape the CLI used for that.
/// That was the honest arrangement when there was no envelope, and it
/// asked every reader to recognise a failure by inspecting it.
///
/// A status says it instead, before the body starts, in the same
/// vocabulary
/// [`OciProxy`](super::oci_proxy::OciProxy) already answers in.
///
/// What a status cannot do is report a failure that arrives partway
/// through a body — the head has gone, and HTTP has no way to take one
/// back. So a body that breaks ends, and a plugin sees a body that
/// stopped, which is what it would see from an ordinary HTTP connection
/// that dropped.
///
/// # The rule it relies on is the specification's, not the frame's
///
/// Nothing in [`ServerFrame`](crate::frame::server::ServerFrame) stops
/// a second channel request arriving on a channel that already has one.
/// This signature says there is only ever one because the endpoint says
/// so, which means a dispatcher needs a stated policy for a frame that
/// should not exist rather than a place to put it.
pub trait CommandProxy: Send + Sync {
    /// Run one command, and answer it.
    ///
    /// The head goes back first and is never repeated; the body follows
    /// it, as one piece or as many. See [`Body`] for why that choice is
    /// the caller's and not this layer's.
    ///
    /// # The request can borrow
    ///
    /// It could not before. The old signature took the command by
    /// value, because the answers could outlast the task that received
    /// the frame by as long as the command ran, and a borrow cannot
    /// cross that.
    ///
    /// Nothing outlasts the call now. This resolves once the head is
    /// known and the running shows up in the body, which owns whatever
    /// it needs — so the request only has to survive until the head
    /// does, exactly as it does for an
    /// [`McpProxy`](super::mcp_proxy::McpProxy).
    ///
    /// # The future is [`Send`]
    ///
    /// Because a plugin may have several commands in flight and a
    /// provider serves several plugins, so these overlap by design. It
    /// is spelled out rather than left to `async fn`, which promises
    /// nothing about the future it returns.
    ///
    /// It resolves when the command has STARTED, not when it has
    /// finished — what it resolves to is a head and a body, and the
    /// running shows up in the second.
    fn handle(
        &self,
        request: request::Request<'_>,
    ) -> impl Future<Output = (response::Head, Body)> + Send;
}

/// The body of an answer: all of it, or a piece at a time.
///
/// # Why the shape is a choice at all
///
/// Because a command is either. One that answers with a document has
/// produced it before it says anything, and one that yields a thousand
/// rows has not — and a proxy forced to pick the first would buffer the
/// second entirely before answering any of it.
///
/// Everything was the second before this existed, because one command
/// somewhere needed it. That made a one-line answer a stream of one,
/// which is not wrong and is not what it is.
///
/// # Both end the same way
///
/// The channel finishes when the body does: after the one piece, or
/// after the stream yields [`None`]. Nothing else says the command is
/// over, and nothing needs to.
///
/// Which is also all a stream can do about its own failure. There is
/// nowhere to report one after the head has gone — the status is
/// already sent, and HTTP has no way to take it back — so a command
/// that breaks mid-body ends the stream, and the plugin sees a body
/// that stopped.
///
/// # It is written again rather than shared
///
/// [`McpProxy`](super::mcp_proxy::McpProxy) and
/// [`OciProxy`](super::oci_proxy::OciProxy) each carry one of these
/// too. They are separate on purpose: these are three traits a provider
/// implements separately, and one shared type would make a change made
/// for one of them a change to all three.
pub enum Body {
    /// The whole answer, at once.
    ///
    /// For a command that produced its answer before saying anything.
    /// It goes out as a single body frame.
    Single(Bytes),
    /// The answer a piece at a time, for as long as it lasts.
    ///
    /// For a command that yields as it goes. Each item is one body
    /// frame, sent as it arrives, and the channel finishes when the
    /// stream does.
    ///
    /// # Why it is boxed, and why the bounds are what they are
    ///
    /// [`Send`] and `'static` because it outlives the call that made it
    /// and will be polled from wherever the answer is being written,
    /// which is not where it was built.
    ///
    /// [`Sync`] is NOT required. Whoever writes the answer OWNS this
    /// and polls it through `&mut`, so a shared reference to it never
    /// exists — and requiring one turns away the obvious way to write a
    /// stream, since an `async_stream` generator is [`Sync`] only if
    /// everything it awaits is. A mutex guard held across an await is
    /// enough to disqualify it.
    Stream(Pin<Box<dyn Stream<Item = Bytes> + Send + 'static>>),
}
