//! Answering the registry pulls a provider forwards.

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::Stream;

use crate::shared::http::request;
use crate::shared::http::response;

/// What serves the images a provider is asked for.
///
/// A provider opens a registry channel because a container runtime
/// beside it is pulling an image the provider does not hold. The image
/// lives with the caller, so the request comes out and this is what a
/// caller implements to answer it.
///
/// It is opened only for an
/// [`Image::Client`](crate::shared::container::request::Image::Client),
/// and opened by the RUNTIME's appetite rather than the provider's: the
/// provider stands up a registry endpoint, the runtime pulls from it,
/// and every request the runtime makes that the provider cannot answer
/// becomes one of these.
///
/// # It serves two endpoints
///
/// Both an [`mcp plugin run`](crate::endpoints::mcp_plugin::run::server::channel_request::Frame::Oci)
/// and a [`laboratory run`](crate::endpoints::laboratories::run::server::channel_request::Frame::Oci)
/// ask for an image this way, in the same words. Which is why it sits
/// here beside the other proxies rather than under either of them.
///
/// # It is a registry, and the semantics are load-bearing
///
/// Not a byte pipe. This specification RELIES on what the distribution
/// protocol means: `404` says a blob is absent, `206` resumes a partial
/// pull, `HEAD` probes without transferring, `Range` picks up where a
/// broken transfer stopped. An implementation that answered every
/// request with `200` and the bytes would be a correct HTTP server and a
/// broken registry — a runtime's cache is the only cache here, and it
/// caches on what these statuses say.
///
/// The provider understands none of it, which is what makes that work.
/// It does not parse a manifest to find layers, does not diff digests,
/// and does not decide what a blob is.
///
/// # One endpoint, every run at once
///
/// The repository segment of the path names the scope, so a request
/// routes itself and a caller serving many runs does not have to keep
/// state between them. What identifies the run is in the request.
///
/// # Failure is a status, not a [`Result`]
///
/// There is no error variant on this channel, and this returns no
/// [`Result`], for the same reason: the exchange is HTTP, and the
/// distribution protocol already says how things go wrong. A caller that
/// cannot reach its store answers `502`; one asked for a blob it does
/// not have answers `404`, which a runtime reads as a fact about the
/// image rather than a fault. A second failure vocabulary beside those
/// would be two ways to say one thing, and a reader would have to check
/// both.
///
/// Which is what each of these traits does, into a different vocabulary
/// each time — see [`PostgresProxy`](super::postgres_proxy::PostgresProxy),
/// which answers in pgwire, and
/// [`CommandProxy`](super::command_proxy::CommandProxy), which answers
/// in the CLI's own.
///
/// So there is always an answer. What varies is what it says.
///
/// # Why it is not [`McpProxy`](super::mcp_proxy::McpProxy)
///
/// The two have the same shape, and nearly nothing else. One documents
/// JSON-RPC, sessions and event streams; this one documents blobs,
/// digests and resumption. Merged, every reader would read half a page
/// that does not apply to them.
///
/// A shared supertrait would be worse than merging rather than better: a
/// type gets one impl of it, so it could serve MCP or images but not
/// both, and a caller doing both is the ordinary case. A marker
/// parameter would work and still lose — bounds would read
/// `P: HttpProxy<Oci>` instead of `P: OciProxy`, and this method's
/// documentation could no longer say what a `404` means.
///
/// The two share a method name deliberately, which is a cost worth
/// naming: on a type implementing both, `handle` is ambiguous and
/// resolves as `<T as OciProxy>::handle`. That does not arise where
/// these are actually called — a dispatcher is generic over one of
/// them at a time, so the bound picks the method — and the alternative
/// was two verbs for one act, which would have made a reader wonder
/// what the difference was.
pub trait OciProxy: Send + Sync {
    /// Take one registry request, and answer it.
    ///
    /// The head goes back first and is never repeated; the body follows
    /// it, as one piece or as many. See [`Body`] for why that choice is
    /// the caller's and not this layer's, and why it matters more here
    /// than anywhere else.
    ///
    /// # The future is [`Send`]
    ///
    /// Because a runtime pulls layers in parallel and a provider serves
    /// several containers at once, so these overlap by design. A task
    /// that cannot move between threads pins all of that to one.
    ///
    /// It is spelled out rather than left to `async fn`, which promises
    /// nothing about the future it returns.
    ///
    /// # The future is not `'static`
    ///
    /// It borrows the request, which borrows the frame it was decoded
    /// from. A dispatcher answering these on their own tasks owns those
    /// bytes inside the task rather than decoding before it — the same
    /// arrangement `agentic_loop`'s already has.
    fn handle(
        &self,
        request: request::Request<'_>,
    ) -> impl Future<Output = (response::Head, Body)> + Send;
}

/// The body of an answer: all of it, or a piece at a time.
///
/// # Why the shape is a choice at all
///
/// Because the two things a registry answers are not the same size. A
/// manifest is a small JSON document that is complete before it is sent.
/// A layer is a blob, routinely hundreds of megabytes, and a caller
/// reading one out of object storage has it as a stream long before it
/// has it as a [`Vec`].
///
/// A proxy that had to pick one would have to buffer the second into the
/// first — holding a whole layer in memory per concurrent pull, and
/// answering nothing until the last byte of it arrived.
///
/// # Both end the same way
///
/// The channel finishes when the body does: after the one piece, or
/// after the stream yields [`None`]. A caller with nothing more to say
/// says nothing more, and the finish is what says so.
///
/// Which is also all a stream can do about its own failure. There is
/// nowhere to report one after the head has gone — the status is already
/// sent, and HTTP has no way to take it back — so a proxy whose storage
/// breaks mid-layer ends the stream, and the runtime sees a body that
/// stopped short of its `Content-Length`. That is the same thing it
/// would see from an ordinary registry whose connection dropped, which
/// is what this is standing in for, and a runtime already knows to
/// retry it.
///
/// # It is its own type, not
/// [`mcp_proxy::Body`](super::mcp_proxy::Body)
///
/// They are identical today. Two endpoints happening to relay the same
/// thing is a fact about them rather than a shared abstraction one of
/// them should own — and the day one grows a variant the other has no
/// use for, sharing would be the thing standing in the way. Told apart
/// by their module, the way every `Frame` in this crate is.
pub enum Body {
    /// The whole answer, at once.
    ///
    /// For a manifest, a tag list, or any of the small structured
    /// documents the protocol is mostly made of. It goes out as a single
    /// body frame.
    Single(Bytes),
    /// The answer a piece at a time, for as long as it lasts.
    ///
    /// For a blob. Each item is one body frame, sent as it arrives, and
    /// the channel finishes when the stream does — so a layer moves
    /// through the caller rather than accumulating in it.
    ///
    /// # Why it is boxed, and why the bounds are what they are
    ///
    /// [`Send`] and `'static` because it outlives the call that made it
    /// and will be polled from wherever the answer is being written,
    /// which is not where it was built.
    ///
    /// [`Sync`] because it is held behind a shared reference while that
    /// happens. It is the strictest of the three and the one most likely
    /// to bite: a stream needs only `&mut` to be polled, so plenty of
    /// otherwise ordinary ones are [`Send`] without being [`Sync`].
    ///
    /// The type is written out rather than aliased, here and in the two
    /// other proxies that return one. What an alias would buy is forty
    /// characters; what it would cost is hiding exactly those three
    /// bounds from the signature a reader is checking theirs against.
    Stream(Pin<Box<dyn Stream<Item = Bytes> + Send + Sync + 'static>>),
}
