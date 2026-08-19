//! Starting a watch, and reading it for as long as it lasts.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::stream::FusedStream;
use tokio::sync::mpsc::Receiver;

use super::request;
use crate::client::handle::Handle;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::volumes::watch::server::response;
use crate::frame;
use crate::shared::error::Error;
use crate::shared::filetree;

/// Start watching a volume.
///
/// # Why this one does not hand back an answer
///
/// Every other volume endpoint collapses into a single call: one
/// question, one answer, then the scope is over, so there is a value to
/// return. A watch does not end. It sends a snapshot and then the
/// changes to it for as long as the scope stays open, and something
/// that returned one value would have had to pick a frame and throw the
/// rest away.
///
/// So this returns a [`Watch`] instead — the same saving the others
/// make, made once per frame rather than once.
///
/// # It fails in only one way
///
/// The request either serializes or it does not. Everything after that
/// belongs to the watch rather than to the asking — a provider that
/// refuses is refusing the watch, and it says so in a frame like
/// everything else. See [`WatchError`].
///
/// # Choosing the capacity
///
/// It belongs to the caller because only the caller knows what it is
/// watching. A directory nobody touches sends a snapshot and goes
/// quiet; one under a build sends thousands of frames a second.
///
/// The unit is whole frames, headers included, and a frame is as large
/// as whatever the provider chunked — the memory is its choice of
/// chunk, not this one's.
///
/// Depth buys tolerance for a reader that falls behind and costs memory
/// while it does. Too shallow loses nothing — the router waits rather
/// than dropping — but it waits for every scope on the connection, not
/// just this one. Zero is not allowed and panics inside
/// [`Handle::send_request`], after the request has been encoded and
/// before anything reaches the wire, which is
/// [`tokio`](tokio::sync::mpsc::channel)'s rule rather than this one.
///
/// The second capacity is `1` and not offered. Nothing is supposed to
/// open a channel inside a watch, and what would carry one is dropped
/// before this returns.
pub async fn execute(
    handle: &Handle,
    request: &request::Frame,
    capacity: usize,
) -> Result<Watch, ExecuteError> {
    let mut payload = Vec::new();
    request
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let scope = handle.send_request(&payload, capacity, 1).await;
    // Only the responses are kept. See `Watch` for why keeping the rest
    // would be a connection-wide hazard rather than a spare field.
    Ok(Watch {
        responses: Some(scope.response_receiver),
    })
}

/// A watch that is running, and the changes coming out of it.
///
/// What [`execute`] gives back, and a [`Stream`] of what the provider
/// says about the tree. The request has gone out and the router is
/// already putting frames where this will find them.
///
/// The item is [`filetree::response::Frame`], the SHARED one — a
/// snapshot and then changes to it. Stripping this endpoint's own
/// envelope off it is most of what this type is for. What that sequence
/// means, and what a reader has to hold to make sense of it, is
/// [`filetree`]'s to say.
///
/// # Zero or more answers, then one ending
///
/// It yields zero or more [`Ok`], and then either ends or yields
/// exactly one [`Err`] and ends. Every error is terminal, which is what
/// makes the type explainable in one line and what
/// [`FusedStream`] then reports honestly.
///
/// Terminal for the errors that are, not by convention. A payload that
/// will not decode is a hole in a FOLD — the frames apply to a tree the
/// reader is keeping, so one missing removal leaves that tree
/// permanently wrong with no way to notice. Carrying on would be
/// applying changes to something known to be broken. The recovery is a
/// new watch and a fresh snapshot, which is cheap.
///
/// # It holds the responses and nothing else
///
/// A [`Scope`](crate::client::scope::Scope) carries a second receiver,
/// for the channels a provider opens inside it. Nothing is supposed to
/// open one inside a watch — but nothing forbids it either, and that
/// receiver is bounded at `1` while the router AWAITS it. Holding one
/// unread would mean a provider that opened two channels blocked the
/// router forever, stalling every scope on the connection.
///
/// So [`execute`] keeps the response receiver and lets the rest go, and
/// a stray channel request dead-letters. Which is the rule
/// [`Scope`](crate::client::scope::Scope) states about itself: drop what
/// you are not going to read.
///
/// # Reading is not optional
///
/// The queue is bounded, at whatever depth [`execute`] was given. A
/// watch nobody reads stops the router that many frames later, and
/// stopping the router stops every scope on the connection rather than
/// only this one.
///
/// That obligation gets sharper as a [`Stream`], not softer. A
/// `select!` is exactly where a stream someone means to ignore ends
/// up, and one parked there that rarely wins stalls the connection
/// once `capacity` frames pile up.
///
/// # Dropping this stops you reading, not the provider writing
///
/// There is no frame for cancelling a scope. A client opens one and a
/// server ends one; nothing in
/// [`ClientFrame`](crate::frame::client::ClientFrame) says stop. So a
/// dropped [`Watch`] leaves the provider watching and sending, the
/// router decoding and discarding a frame at a time, and the scope
/// number unreclaimed — because that only happens on a finish which is
/// never coming.
///
/// For every other endpoint that is the rare case of a caller walking
/// away. For a watch it is the ORDINARY exit, because a watch never
/// ends by itself. It is a gap in the protocol rather than in this
/// type, and it is the strongest argument this crate has for a
/// cancel frame.
#[must_use = "a Watch that is not polled stalls every scope on the connection"]
#[derive(Debug)]
pub struct Watch {
    /// The scope's responses, until there are no more.
    ///
    /// [`None`] once the stream has ended, which is the terminal state
    /// and the whole of it. An [`Option`] rather than a flag beside a
    /// receiver, because taking it out is what a flag would only
    /// record: the queue is dropped the moment nothing will read it, so
    /// its frames are freed and the router's later sends fail at once
    /// instead of filling something nobody is holding.
    ///
    /// It also has to be one or the other.
    /// [`poll_recv`](Receiver::poll_recv) latches [`None`] forever once
    /// the senders are gone, so a stream that reported that as an error
    /// each time it saw it would report it without end.
    responses: Option<Receiver<Bytes>>,
}

/// One change on the tree at a time, until there are no more.
///
/// [`None`] is the watch ending as it should: a finish frame, meaning
/// the provider will not send another. [`WatchError::Closed`] is the
/// connection going away mid-watch, and the two are worth telling apart
/// — one says the tree is no longer being reported, the other says
/// nothing at all, so what the tree looks like now is unknown rather
/// than unchanged.
///
/// There is no timeout. A tree nobody is touching is a watch that says
/// nothing for as long as that lasts, and a quiet stream is not a
/// finished one.
impl Stream for Watch {
    type Item = Result<filetree::response::Frame, WatchError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // Every field is `Unpin` — one channel end — so this never has
        // to project.
        let this = self.get_mut();
        let Some(responses) = this.responses.as_mut() else {
            return Poll::Ready(None);
        };
        let Some(bytes) = ready!(responses.poll_recv(cx)) else {
            this.responses = None;
            return Poll::Ready(Some(Err(WatchError::Closed)));
        };
        let envelope = match frame::server::ServerFrame::decode(&bytes) {
            Ok(envelope) => envelope,
            Err(error) => {
                this.responses = None;
                return Poll::Ready(Some(Err(WatchError::Frame(error))));
            }
        };
        // A router puts only a response and its finish on a scope's
        // response stream, so anything else is the finish — the watch
        // ending, which is what `None` says.
        let frame::server::ServerFrame::Response { payload, .. } = envelope
        else {
            this.responses = None;
            return Poll::Ready(None);
        };
        Poll::Ready(Some(match response::Frame::decode(payload) {
            Ok(response::Frame::Filetree(frame)) => Ok(frame),
            Ok(response::Frame::Error(error)) => {
                this.responses = None;
                Err(WatchError::Provider(error))
            }
            Err(error) => {
                this.responses = None;
                Err(WatchError::Response(error))
            }
        }))
    }
}

/// Whether the stream is over.
///
/// Free to answer, because the terminal state is a field rather than
/// something to work out — and worth answering, because a watch is
/// exactly the stream that ends up in a `select!`, which wants a
/// fused one.
impl FusedStream for Watch {
    fn is_terminated(&self) -> bool {
        self.responses.is_none()
    }
}

/// A watch that never started.
///
/// One way, because starting one is only serializing the request and
/// writing it. Everything a provider might object to is objected to
/// afterwards, in a frame — see [`WatchError`], which is the watch that
/// started and then stopped without ending.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request would not serialize.
    Request(postcard::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Request(error) => {
                write!(f, "watch request did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Request(error) => Some(error),
        }
    }
}

/// A watch that stopped without ending.
///
/// None of these is the watch finishing. That is [`None`] from the
/// [`Stream`], and the difference is the whole reason this type exists:
/// a watch that ended told a caller the tree is no longer being
/// reported, and a watch that broke told it nothing.
///
/// Every one of them is the last thing the stream yields.
#[derive(Debug)]
pub enum WatchError {
    /// The connection ended mid-watch.
    ///
    /// The frames stopped without a finish, so what the tree looks like
    /// now is unknown — not unchanged.
    Closed,
    /// What came back was not a frame.
    ///
    /// Unreachable through this crate's own router, which decodes the
    /// same bytes before forwarding them and discards what will not
    /// parse. It is here because
    /// [`Scope`](crate::client::scope::Scope) is public and its
    /// receiver could be fed by something else.
    Frame(frame::FrameError),
    /// The response frame did not parse.
    ///
    /// Terminal, and that is the point: what rides this stream is a
    /// fold, so a frame that cannot be applied leaves the reader's tree
    /// wrong forever. Start a new watch and take the snapshot again.
    Response(response::FrameDecodeError),
    /// The provider stopped watching, and said so.
    ///
    /// An answer of a kind — the provider is saying the watch is over
    /// and why — but not a change to the tree, which is why it is not
    /// the stream simply ending.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Provider(Error),
}

impl fmt::Display for WatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WatchError::Closed => {
                f.write_str("connection ended in the middle of a watch")
            }
            WatchError::Frame(error) => {
                write!(f, "watch frame did not decode: {error}")
            }
            WatchError::Response(error) => {
                write!(f, "watch frame did not parse: {error}")
            }
            WatchError::Provider(_) => {
                f.write_str("the provider stopped watching")
            }
        }
    }
}

impl std::error::Error for WatchError {
    /// [`Provider`](WatchError::Provider) has no source, because what
    /// it carries is not a Rust error and deliberately does not
    /// implement one — see
    /// [`shared::error::Error`](crate::shared::error::Error). A caller
    /// that wants what is inside it matches the variant.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WatchError::Frame(error) => Some(error),
            WatchError::Response(error) => Some(error),
            WatchError::Closed | WatchError::Provider(_) => None,
        }
    }
}
