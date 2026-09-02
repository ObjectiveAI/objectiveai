//! Answering the fetches a provider forwards.

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::Stream;

/// What answers a provider asking for content it does not hold.
///
/// An agent's request names content by identity — files in
/// `file_mounts`, directories in `directory_mounts`, resources on
/// the agent's own `*_resource` fields — and the bytes live in the
/// caller's own store; the continuation a run resumes from lives
/// there too, unnamed, because there is only ever one. When a
/// provider needs any of them, the ask comes out as a
/// [`FetchFile`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchFile),
/// [`FetchDirectory`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchDirectory),
/// [`FetchResource`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchResource)
/// or
/// [`FetchContinuation`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchContinuation)
/// channel request, and this is what a caller implements to answer
/// them.
///
/// # Four methods, because there are four exchanges
///
/// The identity says which content — not the mount path or the
/// field, which are the caller's placement and which the provider
/// never asks by. The items are OWNED ([`Bytes`], and a path beside
/// it for a directory's files); the executor borrows each into the
/// exchange's frame as it writes, so nothing is copied on the way
/// out.
///
/// # The sender chunks; the receiver never has to know
///
/// No yielded item's bytes may exceed
/// [`CHUNK_SIZE`](crate::endpoints::agentic_loop::run::client::channel_response::CHUNK_SIZE):
/// a larger file is yielded as consecutive items — same path for a
/// directory's file, bare adjacency for the file exchange — and the
/// far side reassembles by appending, never measuring. Only split
/// what exceeds the chunk size: an empty item is a legitimately
/// empty file, not a continuation.
///
/// # The answer is a stream, and its end means whole
///
/// The stream ending is what says the content is complete, and an
/// EMPTY stream is the whole of how an implementation says it does
/// not hold the identity — the channel finishes with nothing on it,
/// which is this protocol's deliberate could-not-serve. There is no
/// error vocabulary on these exchanges: an implementation that fails
/// partway ends the stream, and the far side finds out because a
/// partial answer neither measures nor hashes to the identity it
/// asked for.
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
    /// The file behind one identity, as its bytes — one item per
    /// chunk, appended in order by the far side.
    ///
    /// # The future is [`Send`]
    ///
    /// Because a provider serving several containers asks at once, and
    /// a task that cannot move between threads pins that concurrency
    /// to one. It is spelled out rather than left to `async fn`, which
    /// promises nothing about the future it returns.
    fn fetch_file(
        &self,
        identity: String,
    ) -> impl Future<
        Output = Pin<Box<dyn Stream<Item = Bytes> + Send + 'static>>,
    > + Send;

    /// The directory behind one identity, one file — or one chunk of
    /// one, path repeated — at a time. The path is relative to the
    /// directory, one component per element, the final element the
    /// filename.
    ///
    /// # The future is [`Send`]
    ///
    /// For the reason [`fetch_file`](Self::fetch_file) gives.
    fn fetch_directory(
        &self,
        identity: String,
    ) -> impl Future<
        Output = Pin<
            Box<dyn Stream<Item = (Vec<String>, Bytes)> + Send + 'static>,
        >,
    > + Send;

    /// The resource behind one identity, as its bytes — one item per
    /// chunk, appended in order by the far side. The identity is the
    /// FILE grammar (`f1:…`); whether an identity names a mounted
    /// file or a resource is the store's business, not the wire's —
    /// content-addressing does not care what the bytes are for.
    ///
    /// # The future is [`Send`]
    ///
    /// For the reason [`fetch_file`](Self::fetch_file) gives.
    fn fetch_resource(
        &self,
        identity: String,
    ) -> impl Future<
        Output = Pin<Box<dyn Stream<Item = Bytes> + Send + 'static>>,
    > + Send;

    /// The continuation the run resumes from, as its bytes — one
    /// item per chunk, appended in order by the far side. No
    /// identity: a run resumes from the one continuation its caller
    /// holds. An EMPTY stream is a fresh start, the ordinary first
    /// run — not a refusal.
    ///
    /// # The future is [`Send`]
    ///
    /// For the reason [`fetch_file`](Self::fetch_file) gives.
    fn fetch_continuation(
        &self,
    ) -> impl Future<
        Output = Pin<Box<dyn Stream<Item = Bytes> + Send + 'static>>,
    > + Send;
}
