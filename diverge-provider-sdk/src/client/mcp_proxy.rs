//! Answering the MCP exchanges a provider forwards.

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::Stream;

use crate::shared::http::request;
use crate::shared::http::response;

/// What answers an MCP request a provider could not.
///
/// A provider opens an MCP channel because something inside a container
/// wants to talk to a server the provider cannot reach — the MCP
/// servers live with the caller. So the request comes out, and this is
/// what a caller implements to answer it.
///
/// # It is a proxy, not a server
///
/// Nothing here parses JSON-RPC, tracks a session, or reads the
/// `Mcp-Session-Id` that ties a caller's exchanges together. What
/// arrives is an HTTP request; what goes back is an HTTP response; the
/// thing in the middle is somebody else's MCP server. An implementation
/// that does understand MCP is welcome to, and nothing in this
/// specification will notice.
///
/// # Failure is a status, not a [`Result`]
///
/// There is no error variant on an MCP channel, and this returns no
/// [`Result`], for the same reason: the exchange is HTTP, and HTTP
/// already says how things go wrong. A proxy that cannot reach its
/// server answers `502`; one that is asked for something that is not
/// there answers `404`. A second failure vocabulary beside those would
/// be two ways to say one thing, and a reader would have to check both.
///
/// So there is always an answer. What varies is what it says.
pub trait McpProxy: Send + Sync {
    /// Take one request, and answer it.
    ///
    /// The head goes back first and is never repeated; the body follows
    /// it, as one piece or as many. See [`Body`] for why that choice is
    /// the caller's and not this layer's.
    ///
    /// # The future is [`Send`]
    ///
    /// Because a provider serving several containers answers their
    /// exchanges at once, and a task that cannot move between threads
    /// pins that concurrency to one. It is spelled out rather than left
    /// to `async fn`, which promises nothing about the future it
    /// returns.
    fn handle(
        &self,
        request: request::Request<'_>,
    ) -> impl Future<Output = (response::Head, Body)> + Send;
}

/// The body of an answer: all of it, or a piece at a time.
///
/// # Why the shape is a choice at all
///
/// Because MCP over Streamable HTTP answers in two ways, and the head
/// says which. A `Content-Type` of `application/json` introduces one
/// document, complete, and a request that got one is over. A
/// `text/event-stream` introduces an event stream held open for the
/// session, which is where a server pushes notifications and the
/// answers to things it was asked while it was thinking.
///
/// A proxy that had to pick one would have to buffer the second into
/// the first — which for a stream held open for a session means
/// buffering forever, and answering nothing until it closed.
///
/// # Both end the same way
///
/// The channel finishes when the body does: after the one piece, or
/// after the stream yields [`None`]. A caller with nothing more to say
/// says nothing more, and the finish is what says so.
///
/// Which is also all a stream can do about its own failure. There is
/// nowhere to report one after the head has gone — the status is
/// already sent, and HTTP has no way to take it back — so a proxy that
/// breaks mid-body ends the stream, and the far end sees a body that
/// stopped. That is the same thing it would see from an ordinary HTTP
/// connection that dropped, which is what this is standing in for.
pub enum Body {
    /// The whole answer, at once.
    ///
    /// For the ordinary case: one JSON document, complete before it was
    /// sent. It goes out as a single body frame.
    Single(Bytes),
    /// The answer a piece at a time, for as long as it lasts.
    ///
    /// For an event stream. Each item is one body frame, sent as it
    /// arrives, and the channel finishes when the stream does.
    ///
    /// # Why it is boxed, and why the bounds are what they are
    ///
    /// [`Send`] and `'static` because it outlives the call that made it
    /// and will be polled from wherever the answer is being written,
    /// which is not where it was built.
    ///
    /// [`Sync`] is NOT required, and used to be. Whoever writes the
    /// answer OWNS this and polls it through `&mut`, so a shared
    /// reference to it never exists — and requiring one turned away the
    /// obvious way to write a stream, since an `async_stream` generator
    /// is [`Sync`] only if everything it awaits is. A mutex guard held
    /// across an await was enough to disqualify it.
    Stream(Pin<Box<dyn Stream<Item = Bytes> + Send + 'static>>),
}
