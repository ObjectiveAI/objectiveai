//! One frame ahead: how a reader keeps its sender from outrunning it.
//!
//! Every stream on a connection — a scope's responses, a scope's
//! channel requests, a channel's responses, in either direction — is
//! paced by its reader. The rules:
//!
//! 1. Every stream opens with room for exactly one frame.
//! 2. A sender sends a response, a channel request or a channel
//!    response only while it has room, and sending spends it. Without
//!    room the send waits ([`Window::take`]); only that stream waits, and
//!    the connection carries every other one meanwhile.
//! 3. A finish needs no room and never waits: a reader's queue holds at
//!    most one frame and the finish behind it ([`DEPTH`]).
//! 4. The reader gives room back ([`Kind::Responses`],
//!    [`Kind::Requests`]) when its consumer TAKES a frame off the queue,
//!    not when the frame is filed. One frame is being worked on while the
//!    next waits; that is the whole of "one ahead".
//! 5. A reader that is dropped before its stream's finish says so
//!    ([`Kind::ResponsesAbandoned`], [`Kind::RequestsAbandoned`]). From
//!    then on its sender's sends on that stream return at once and send
//!    nothing, and a frame already on its way is discarded on arrival.
//! 6. A frame that arrives on a stream whose queue is full broke rule 2,
//!    and ends the connection. A credit for a stream the sender does not
//!    know — one that already ended — is nothing.
//! 7. A scope that finishes, or a connection that ends, ends every
//!    window under it, and a send waiting on one returns ([`Ended`]).
//! 8. Opening a scope is not paced: a server pulls requests by polling
//!    its session.
//!
//! Which bounds memory, everywhere: what can sit in a queue is one frame
//! and one finish per stream, and a frame is at most
//! [`CHUNK_SIZE`](crate::CHUNK_SIZE) and its header.

use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use tokio::sync::mpsc::{self, UnboundedSender, error::TryRecvError};
use tokio::sync::watch;

/// How many frames a reader's queue holds: one frame, and its finish
/// (rule 3).
pub const DEPTH: usize = 2;

/// What a credit says, as the one byte of its payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Room for one more frame on the response stream the header names:
    /// channel `0` for the scope's own responses, `N` for channel `N`'s.
    Responses = 0,
    /// Room for one more channel request in the scope.
    Requests = 1,
    /// That response stream's reader reads nothing more.
    ResponsesAbandoned = 2,
    /// The scope's channel requests are read no more.
    RequestsAbandoned = 3,
}

impl Kind {
    /// The kind a byte names; `None` for one this revision does not.
    pub fn of(byte: u8) -> Option<Kind> {
        match byte {
            0 => Some(Kind::Responses),
            1 => Some(Kind::Requests),
            2 => Some(Kind::ResponsesAbandoned),
            3 => Some(Kind::RequestsAbandoned),
            _ => None,
        }
    }
}

/// One credit to send: the stream it is for, and what it says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Credit {
    /// The scope.
    pub scope: u32,
    /// The channel, in the numbering of the end sending the stream; `0`
    /// for the scope's own responses and for its channel requests.
    pub channel: u32,
    /// What it says.
    pub kind: Kind,
}

/// Why a send that waited will not happen (rule 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    /// The scope finished.
    Scope,
    /// The connection ended.
    Connection,
}

/// What a sender may do now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Go {
    /// Send the frame: its room is spent.
    Send,
    /// Send nothing: the reader is gone (rule 5).
    Skip,
}

#[derive(Debug)]
struct State {
    room: u32,
    abandoned: bool,
    ended: Option<Ended>,
}

/// One outgoing stream's room, shared by whoever sends on it and
/// whoever routes its credits. Cheap to clone.
#[derive(Debug, Clone)]
pub struct Window(Arc<watch::Sender<State>>);

impl Default for Window {
    fn default() -> Self {
        Window::new()
    }
}

impl Window {
    /// A stream's window as it opens: room for one frame (rule 1).
    pub fn new() -> Self {
        Window(Arc::new(watch::Sender::new(State { room: 1, abandoned: false, ended: None })))
    }

    /// Wait for room, and spend it (rule 2); or learn there is no reader
    /// (rule 5) or no stream (rule 7). Cancel-safe: dropped while
    /// waiting, nothing was spent.
    pub async fn take(&self) -> Result<Go, Ended> {
        let mut changes = self.0.subscribe();
        loop {
            let mut verdict = None;
            self.0.send_if_modified(|state| {
                if let Some(ended) = state.ended {
                    verdict = Some(Err(ended));
                    false
                } else if state.abandoned {
                    verdict = Some(Ok(Go::Skip));
                    false
                } else if state.room > 0 {
                    state.room -= 1;
                    verdict = Some(Ok(Go::Send));
                    true
                } else {
                    false
                }
            });
            if let Some(verdict) = verdict {
                return verdict;
            }
            if changes.changed().await.is_err() {
                return Err(Ended::Connection);
            }
        }
    }

    /// A credit arrived: what it says, applied.
    pub fn apply(&self, kind: Kind) {
        match kind {
            Kind::Responses | Kind::Requests => self.0.send_modify(|state| state.room += 1),
            Kind::ResponsesAbandoned | Kind::RequestsAbandoned => self.0.send_modify(|state| state.abandoned = true),
        }
    }

    /// Rule 7. The first ending stands.
    pub fn end(&self, why: Ended) {
        self.0.send_modify(|state| {
            state.ended.get_or_insert(why);
        });
    }
}

/// Where a reader's credits go: the connection's outbox, and the
/// stream they name.
#[derive(Debug)]
struct Grant {
    outbox: UnboundedSender<Credit>,
    scope: u32,
    channel: u32,
    requests: bool,
}

impl Grant {
    fn post(&self, abandoned: bool) {
        let kind = match (self.requests, abandoned) {
            (false, false) => Kind::Responses,
            (true, false) => Kind::Requests,
            (false, true) => Kind::ResponsesAbandoned,
            (true, true) => Kind::RequestsAbandoned,
        };
        let _ = self.outbox.send(Credit { scope: self.scope, channel: self.channel, kind });
    }
}

/// Whether a whole frame is a finish: type `3` or `6`, the only two
/// that end a stream, in either direction.
fn finishes(frame: &Bytes) -> bool {
    matches!(frame.first(), Some(3 | 6))
}

/// The router's end of a reader's queue.
pub type Filer = mpsc::Sender<Bytes>;

/// A reader of one stream: whole frames, header included, in the order
/// they arrived, with its credits sent as rules 4 and 5 say.
///
/// Read with [`recv`](Self::recv), [`poll_recv`](Self::poll_recv) or
/// [`try_recv`](Self::try_recv), as a tokio receiver is. `None` is the
/// end: after the finish, or with the connection.
#[derive(Debug)]
pub struct Receiver {
    queue: mpsc::Receiver<Bytes>,
    grant: Grant,
    done: bool,
}

impl Receiver {
    /// A reader and the router's end of it, for the stream `(scope,
    /// channel)` — its channel requests if `requests` — crediting
    /// through `outbox`.
    pub fn new(outbox: UnboundedSender<Credit>, scope: u32, channel: u32, requests: bool) -> (Filer, Receiver) {
        let (filer, queue) = mpsc::channel(DEPTH);
        (filer, Receiver { queue, grant: Grant { outbox, scope, channel, requests }, done: false })
    }

    /// Rule 4 for a frame taken, and the end noted.
    fn took(&mut self, taken: &Option<Bytes>) {
        if self.done {
            return;
        }
        match taken {
            Some(frame) if finishes(frame) => self.done = true,
            Some(_) => self.grant.post(false),
            None => self.done = true,
        }
    }

    /// The next frame, waiting for it.
    pub async fn recv(&mut self) -> Option<Bytes> {
        let taken = self.queue.recv().await;
        self.took(&taken);
        taken
    }

    /// The next frame, as a poll.
    pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<Bytes>> {
        let polled = self.queue.poll_recv(cx);
        if let Poll::Ready(taken) = &polled {
            self.took(taken);
        }
        polled
    }

    /// The next frame if one is waiting.
    pub fn try_recv(&mut self) -> Result<Bytes, TryRecvError> {
        let tried = self.queue.try_recv();
        match &tried {
            Ok(frame) => self.took(&Some(frame.clone())),
            Err(TryRecvError::Disconnected) => self.done = true,
            Err(TryRecvError::Empty) => {}
        }
        tried
    }

    /// Read nothing more (rule 5): the sender is told, and what is
    /// already queued may still be read.
    pub fn close(&mut self) {
        self.queue.close();
        if !self.done {
            self.done = true;
            self.grant.post(true);
        }
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        if !self.done {
            self.grant.post(true);
        }
    }
}

/// File a frame into a reader's queue (rules 5 and 6): `Err` if the
/// queue was full, which ends the connection; a reader that is gone
/// takes nothing, which is fine.
pub fn file(filer: &Filer, frame: Bytes) -> Result<(), Overrun> {
    match filer.try_send(frame) {
        Ok(()) | Err(mpsc::error::TrySendError::Closed(_)) => Ok(()),
        Err(mpsc::error::TrySendError::Full(_)) => Err(Overrun),
    }
}

/// A frame arrived with no room for it (rule 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overrun;

impl std::fmt::Display for Overrun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a frame arrived on a stream with no room for it")
    }
}

impl std::error::Error for Overrun {}
