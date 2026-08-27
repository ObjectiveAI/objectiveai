//! Answering the fetches a provider forwards.

use std::future::Future;
use std::pin::Pin;

use futures_util::Stream;

use crate::endpoints::agentic_loop::run::client::channel_response::fetch::Frame;
use crate::endpoints::agentic_loop::run::server::channel_request::fetch::Kind;

/// What answers a provider asking for content it does not hold.
///
/// An agent's request names its skills and its agent definitions by
/// dirhash, and the content lives in the caller's own folders. When a
/// provider is missing one, the ask comes out as a
/// [`Fetch`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::Fetch)
/// channel request, and this is what a caller implements to answer it.
///
/// # One method, because there is one exchange
///
/// The kind says which folder family to look in; the dirhash says
/// which content. Not the name — a name is the caller's own label,
/// and the provider never learned it.
///
/// # It names the endpoint's own types
///
/// [`Kind`] and [`Frame`] are the agentic loop's, not
/// [`shared`](crate::shared)'s, because nothing but an agentic loop
/// fetches — the same argument [`postgres_proxy`](super::postgres_proxy)
/// makes for naming a plugin's request.
///
/// # The answer is a stream, and its end means whole
///
/// One [`Frame`] per file, however the implementation orders them.
/// The stream ending is what says the directory is complete, and an
/// EMPTY stream is the whole of how an implementation says it does
/// not hold the hash — the channel finishes with nothing on it, which
/// is this protocol's deliberate could-not-serve. There is no error
/// vocabulary on this exchange: an implementation that fails partway
/// ends the stream, and the far side finds out because a partial set
/// does not hash to the identity it asked for.
///
/// # Why it is boxed, and why the bounds are what they are
///
/// [`Send`] and `'static` because the stream outlives the call that
/// made it and will be polled from wherever the answer is being
/// written, which is not where it was built.
///
/// [`Sync`] is NOT required. Whoever writes the answer OWNS this and
/// polls it through `&mut`, so a shared reference to it never exists —
/// and requiring one turns away the obvious way to write a stream,
/// since an `async_stream` generator is [`Sync`] only if everything it
/// awaits is.
pub trait FetchProxy: Send + Sync {
    /// The directory behind one dirhash, one file at a time.
    ///
    /// # The future is [`Send`]
    ///
    /// Because a provider serving several containers asks at once, and
    /// a task that cannot move between threads pins that concurrency
    /// to one. It is spelled out rather than left to `async fn`, which
    /// promises nothing about the future it returns.
    fn fetch(
        &self,
        kind: Kind,
        dirhash: String,
    ) -> impl Future<
        Output = Pin<Box<dyn Stream<Item = Frame> + Send + 'static>>,
    > + Send;
}
