//! One scope, and everything a provider does inside it.

use std::sync::Arc;

use bytes::Bytes;
use futures_util::stream::SplitSink;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::connection::Connection;

/// A scope a client opened, and the means to answer it.
///
/// What a [`Session`](super::session::Session) yields. The scope is
/// open from the moment this exists — the request has arrived, and the
/// session already knows where to put what follows it.
///
/// This is the half that writes. A [`Session`](super::session::Session)
/// reads a socket and hands out scopes; everything a provider actually
/// SAYS is said through one of these, which is why it and not the
/// session carries the connection's write half.
///
/// # Not [`Clone`]
///
/// It holds a receiver, and a receiver has one reader. Which is also
/// the honest shape: a scope is one request with one answer, and two
/// parties both answering it would be two parties racing to finish it.
///
/// # Dropping it ends the scope, as far as the session is concerned
///
/// [`Drop`] closes the receiver and says so, in that order, and a
/// [`Session`](super::session::Session) forgets the scope on its next
/// poll. It is a destructor rather than a method because that is what
/// makes it unforgettable: abandoning a scope and ending one
/// deliberately are the same event to the session, and neither can be
/// omitted by a caller who did not read this.
///
/// What the CLIENT sees is another matter, and it is nothing at all: *a
/// stream ends at its finish frame, and nowhere else*, so a scope
/// dropped without one leaves the client's reader waiting until the
/// socket dies. Nothing anywhere will time it out.
///
/// # Nothing is written yet
///
/// The fields are the whole of it. Answering a request, ending a scope,
/// taking the channel requests the client opens, opening one of this
/// end's own — none of it is here, and each of those decisions wants
/// this to exist first.
// Nothing reads them yet, because nothing writes yet. The attribute
// goes when the methods do.
#[allow(dead_code)]
#[derive(Debug)]
pub struct ScopeHandle {
    /// The scope's number, chosen by the CLIENT.
    ///
    /// It is in the header of every frame belonging to this scope, in
    /// both directions. What it is free for afterwards is the client's
    /// business; this end never mints one and never reuses one.
    scope: u32,
    /// The frame that opened the scope, whole.
    ///
    /// Kept rather than decoded, because everything that could be
    /// decoded out of it borrows from it: a
    /// [`ClientRequest`](crate::endpoints::ClientRequest) holds slices
    /// of these bytes, so a handle that owned one would be
    /// self-referential.
    ///
    /// Whole, header included, though nothing here needs the header —
    /// the type is always `1` and the channel always `0`, and the scope
    /// is already a field. It stays because slicing it off is a
    /// decision for whatever hands the payload out, and that is not
    /// written.
    request: Bytes,
    /// The channels the client opens inside this scope.
    ///
    /// Whole frames, and the channel number in each header is the
    /// CLIENT's — it is what an answer has to quote to be understood.
    ///
    /// Unbounded, so that a
    /// [`Session`](super::session::Session) can deliver into it without
    /// waiting — which is what lets a session be a
    /// [`Stream`](futures_util::Stream) at all.
    ///
    /// The obligation that comes back the other way is to READ it. A
    /// scope that never does lets a client pile up channel requests
    /// here without limit, and each one can be a whole MCP request.
    /// Nothing bounds that except serving the scope, or dropping it —
    /// which frees the queue with it.
    ///
    /// It is also how a [`Session`](super::session::Session) tells a
    /// live scope from an ended one. Closing this closes the sender it
    /// kept, and that — not the notice beside it — is what the session
    /// actually checks.
    channel_request_receiver: UnboundedReceiver<Bytes>,
    /// Where to say this scope is over.
    ///
    /// Sent from [`Drop`], and only from there. It carries no
    /// information the sender's own state does not already have; what
    /// it does is tell a [`Session`](super::session::Session) WHICH
    /// number to look at, so that forgetting a scope costs a lookup
    /// rather than a walk of every scope on the connection.
    ///
    /// Unbounded, because a destructor has nothing to await on and
    /// nowhere to report a failure. The only way to fail is a session
    /// that is already gone, which has no map left to correct.
    closed: UnboundedSender<u32>,
    /// The write half of the connection, shared with every other scope
    /// on it.
    ///
    /// To be locked for one frame and never for a stream of them. One
    /// frame is the wire's unit of exclusion, and a scope holding this
    /// across a stream of chunks is a scope that has taken the
    /// connection away from everybody else until it finishes.
    sink: Arc<Mutex<SplitSink<Connection, Bytes>>>,
    /// Where a frame is built, before it is handed to the socket.
    ///
    /// Per scope rather than per connection, which is what keeps
    /// encoding OUT of the critical section: a frame is built here
    /// first, and the lock is taken only for the write. A shared buffer
    /// would have to be locked for both.
    ///
    /// It grows to the largest frame this scope has sent and stops. The
    /// copy at the end is not avoidable — a
    /// [`Sink`](futures_util::Sink) of [`Bytes`] takes ownership of
    /// what it sends, and this buffer is not giving up ownership, which
    /// is the whole point of it. What it saves is the growth: a fresh
    /// [`Vec`] per frame reallocates its way up from nothing every
    /// time.
    buffer: Vec<u8>,
}

impl ScopeHandle {
    /// Take the parts, from the [`Session`](super::session::Session)
    /// that has all of them.
    ///
    /// Not public. A scope exists because a client asked for one, so
    /// the only thing that can honestly make one of these is the thing
    /// reading the requests.
    ///
    /// The buffer is not among them. A scope starts having written
    /// nothing, and there is no other answer a caller could give.
    pub(super) fn new(
        scope: u32,
        request: Bytes,
        channel_request_receiver: UnboundedReceiver<Bytes>,
        closed: UnboundedSender<u32>,
        sink: Arc<Mutex<SplitSink<Connection, Bytes>>>,
    ) -> Self {
        ScopeHandle {
            scope,
            request,
            channel_request_receiver,
            closed,
            sink,
            buffer: Vec::new(),
        }
    }
}

/// Tell the session the scope is over.
///
/// # The receiver closes first
///
/// And the order is what makes the notice safe to act on. A
/// [`Session`](super::session::Session) does not remove a scope because
/// it was told to — it removes one whose sender reads as closed, since
/// the number may since have been reused and removing by number alone
/// would evict the new scope instead.
///
/// Closing here rather than leaving it to the field's own drop is what
/// makes that check true the moment the notice is visible. Fields drop
/// after this body returns, so a notice sent first could be read by the
/// session in the window before the receiver went away — and the scope
/// would still look live, and would never be removed at all.
impl Drop for ScopeHandle {
    fn drop(&mut self) {
        self.channel_request_receiver.close();
        let _ = self.closed.send(self.scope);
    }
}
