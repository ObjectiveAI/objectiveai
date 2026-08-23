//! Running the commands a plugin cannot run itself.

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::Stream;

use crate::shared::error::Error;

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
/// A command that produces one item is a stream of one. Which is not
/// the same as pretending everything streams: there is genuinely one
/// shape here, because a command is a thing that runs and emits, and a
/// short answer is a short run.
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
/// # There is no envelope
///
/// Bytes go up and bytes come back, with nothing around them. There was
/// an HTTP one for a while — a method, a path, headers, a status — and
/// it was carrying nothing: a registry request is worth forwarding
/// whole because a registry SPEAKS a protocol and the far end relies on
/// it, and nothing speaks a command except the CLI, which is at the far
/// end of this relay already.
///
/// So the envelope was one this specification had invented and then had
/// to justify, and what it was for is served by a tag byte instead.
///
/// # Failure is a variant, not a [`Result`]
///
/// The stream's items are `Result<Bytes, Error>` and an [`Err`] is the
/// last thing it yields — it becomes an
/// [`Error`](crate::endpoints::mcp_plugin::run::client::channel_response::command::Frame::Error)
/// frame, and the channel finishes after.
///
/// That an error can be SAID at all is the part worth keeping. Before,
/// a command that failed emitted its failure as one of its items, in
/// whatever shape the CLI used for that — so a plugin could only learn
/// of it by inspecting something it was otherwise meant to pass along,
/// and a plugin that did not know the shape could not learn of it.
///
/// What the error says is still nobody's business here. See
/// [`shared::error::Error`](crate::shared::error::Error).
///
/// # The rule it relies on is the specification's, not the frame's
///
/// Nothing in [`ServerFrame`](crate::frame::server::ServerFrame) stops
/// a second channel request arriving on a channel that already has one.
/// This signature says there is only ever one because the endpoint says
/// so, which means a dispatcher needs a stated policy for a frame that
/// should not exist rather than a place to put it.
pub trait CommandProxy: Send + Sync {
    /// Run one command, and stream back what it produces.
    ///
    /// # The request is owned
    ///
    /// Unlike an [`McpProxy`](super::mcp_proxy::McpProxy) method,
    /// which can borrow because it is answered in the task that
    /// received the frame. A command's answers can outlast that task by
    /// as long as the command runs, so a dispatcher hands the bytes to
    /// something long-lived — and a borrow cannot cross that.
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
    /// It resolves when the command has STARTED, not when it has
    /// finished — what it resolves to is the stream of the command's
    /// items, and that is where the running shows up.
    ///
    /// # The bounds on the stream
    ///
    /// [`Send`] and `'static` because it is polled from wherever the
    /// answer is written, which is not where it was built.
    ///
    /// [`Sync`] is NOT required. Whoever writes the answer owns this
    /// and polls it through `&mut`, so a shared reference to it never
    /// exists — and requiring one turns away the obvious way to write a
    /// stream, since an `async_stream` generator is [`Sync`] only if
    /// everything it awaits is.
    fn run(
        &self,
        command: Bytes,
    ) -> impl Future<
        Output = Pin<
            Box<dyn Stream<Item = Result<Bytes, Error>> + Send + 'static>,
        >,
    > + Send;
}
