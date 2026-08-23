//! Answering the registry pulls a provider forwards.

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::Stream;

/// What answers the registry requests a provider relays.
///
/// A provider deploying an
/// [`Image::Client`](crate::shared::container::request::Image::Client)
/// has no image and no way to get one: the bytes are the caller's. So
/// it stands up a registry endpoint, its container runtime pulls from
/// that, and every request the runtime makes comes out on a channel for
/// this to answer.
///
/// # It is a proxy, and the caller decides what kind
///
/// What arrives is one HTTP request, as the runtime wrote it. What goes
/// back is bytes, which the provider writes onto the runtime's socket
/// in the order they arrive. Nothing in between reads either.
///
/// Which leaves an implementation free to be a real proxy, and it will
/// have to be. A registry commonly answers a blob with a `307` to a
/// CDN, and that URL resolves on the CALLER's network — a runtime on
/// the provider's side cannot follow it. So an implementation either
/// rewrites the `Location` or follows the redirect itself and streams
/// the body back, which is what `nginx` does with `proxy_redirect`.
///
/// The same freedom covers the rest: answering as an open registry
/// while using its own credentials upstream, setting `Host`, dropping
/// the hop-by-hop headers a relay must never forward. None of it is
/// expressible when the shape is fixed by this crate, and all of it is
/// the caller's to get right.
///
/// # It serves the pull half, and only that
///
/// `GET` for manifests and blobs, `HEAD` to probe, and `Range` to
/// resume one that was interrupted. There is no push, no delete and no
/// upload session — which is why a request fits in a single frame,
/// since those are the exchanges that carry a body upward and a blob
/// would not fit in one.
///
/// # What it does not have to understand
///
/// Any of it. It does not parse a manifest to find layers, diff digests
/// against a store, or decide what a blob is — a runtime already
/// indexes layers by digest and already has a cache, so a second one
/// here would be a slower copy of a better one.
///
/// # Failure is a status, not a [`Result`]
///
/// There is no error variant on this channel and this returns no
/// [`Result`], because the exchange is HTTP and HTTP already says how
/// things go wrong. A registry that has nothing answers `404`; one that
/// will not say answers `401`; a proxy that cannot reach its upstream
/// answers `502`, which is what a proxy is for.
///
/// A second failure vocabulary beside those would be two ways to say
/// one thing, and a runtime would have to understand both to learn one
/// of them.
///
/// So there is always an answer. What varies is what it says.
pub trait OciProxy: Send + Sync {
    /// Take one registry request, and stream back the answer.
    ///
    /// # The request is owned
    ///
    /// Unlike an [`McpProxy`](super::mcp_proxy::McpProxy) method, which
    /// can borrow because it is answered in the task that received the
    /// frame. A layer runs to hundreds of megabytes and an answer
    /// outlasts that task by as long as it takes to send, so a
    /// dispatcher hands the bytes to something long-lived — and a
    /// borrow cannot cross that.
    ///
    /// It costs nothing to say so. Slicing the payload out of the
    /// arriving frame with
    /// [`Bytes::slice_ref`](bytes::Bytes::slice_ref) is a refcount and
    /// not a copy.
    ///
    /// What arrives is the payload the runtime sent, with the frame
    /// header and the variant tag already off it: a request line and
    /// headers, ending at the blank line.
    ///
    /// # The future is [`Send`]
    ///
    /// Because a runtime pulls layers in parallel and a provider serves
    /// several runs at once, so these overlap by design. It is spelled
    /// out rather than left to `async fn`, which promises nothing about
    /// the future it returns.
    ///
    /// It resolves when the answer has STARTED, not when it has
    /// finished — what it resolves to is the stream, and a blob's
    /// length shows up there rather than here.
    ///
    /// # The stream is infallible, and the pieces mean nothing
    ///
    /// [`Bytes`] rather than a [`Result`], because everything that can
    /// go wrong has a way of being said IN the bytes — see the type's
    /// own documentation for why the frame has no error variant either.
    ///
    /// Where one piece ends is the implementation's business and
    /// nobody else's. A provider writes them onto a socket in order;
    /// what the runtime sees is a byte stream, not these boundaries.
    ///
    /// A stream that ends early is an answer that was cut off, which is
    /// what a runtime sees from any connection that dropped — and is
    /// the one thing a caller can say by doing nothing.
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
    fn handle(
        &self,
        request: Bytes,
    ) -> impl Future<
        Output = Pin<Box<dyn Stream<Item = Bytes> + Send + 'static>>,
    > + Send;
}
