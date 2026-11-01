//! One connection, as the scopes a client opens on it.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt as _, Stream, StreamExt as _};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::notice::Notice;
use super::received::Received;
use super::scope_handle::ScopeHandle;
use crate::wire::connection::Connection;
use crate::wire::credit::{self, Credit, Ended, Filer, Kind, Overrun, Receiver, Window};
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame::client::ClientFrame;
use crate::wire::frame::server::ServerFrame;

/// A connection's provider side: what a client starts on it.
///
/// A [`Stream`] of [`Received`] — a token, or a request beside
/// the [`ScopeHandle`] that answers it — and that is the whole of a
/// provider's outer loop: take an item, judge or serve it, take the
/// next. See [`Received`] for why there are exactly two kinds and why
/// the ordering rules between them are not enforced here.
///
/// # Why this is not a router
///
/// The caller half has one, and it earns its keep there: a client
/// arranges where a scope's frames will go BEFORE opening it, so
/// something has to hold those arrangements and match arriving frames
/// against them. That is routing, and it is a job.
///
/// Nothing here can arrange a SCOPE. A server does not open one, it is
/// told about one — so what would have been a router is a loop that
/// reads frames and hands out the ones that open something.
///
/// Channels are the other way round, and that is why there is a
/// registration queue: a channel this end opens is this end's to
/// number, and the answer to it has to have somewhere to land before
/// it arrives.
///
/// # Polling this is what runs the connection
///
/// Not merely what accepts from it. Every frame on the socket comes
/// through [`poll_next`](Stream::poll_next), including the ones bound
/// for scopes already being served elsewhere — so a caller that stops
/// polling stops the connection, and a scope waiting on a channel in
/// some other task waits forever while the frames that would feed it
/// sit unread.
///
/// Which makes one rule absolute rather than advisory: **serve a scope
/// in its own task, never inside the loop that accepted it.** Doing the
/// work inline means nothing is reading the socket while it runs, and
/// if that work is itself waiting on the client, neither side moves
/// again.
///
/// # The handshake is done before
///
/// A session is built on a connection whose
/// [`handshake`](crate::wire::handshake) is over, so it sends no token.
/// One that arrives is yielded as [`Received::Auth`], for whoever holds
/// this stream to end the connection on.
#[derive(Debug)]
pub struct Session {
    /// The read half of the connection.
    ///
    /// Split from the write half at construction, because the two are
    /// used at once and from different places: this reads in a loop
    /// while the scopes it handed out write — see
    /// [`connection`](crate::wire::connection) for why a
    /// [`Sink`](futures_util::Sink) needing `&mut` makes that a wire
    /// requirement rather than an implementation detail.
    stream: SplitStream<Connection>,
    /// The write half, shared by every scope on this connection.
    ///
    /// One socket, so one of these however many scopes are open, and a
    /// lock around it because a WebSocket forbids interleaving the
    /// fragments of two messages. One writer at a time is the wire's
    /// rule, not a queue discipline anybody chose.
    ///
    /// The lock is [`tokio`]'s, because the guard is held across the
    /// `await` that writes and a [`std`] guard is not [`Send`] across
    /// one.
    ///
    /// This end never writes. It holds this only to clone into the
    /// scopes it yields, which is the whole reason it takes a
    /// [`Connection`] rather than a [`SplitStream`] — the two halves
    /// have to start together for the scopes to get theirs.
    sink: Arc<Mutex<SplitSink<Connection, Bytes>>>,
    /// Every scope a client has open, and everything routed inside it.
    ///
    /// Nested rather than flattened to a `(scope, channel)` key,
    /// because a scope ending ends everything under it: one removal
    /// drops every queue and ends every window at once. It is the same
    /// shape, for the same reason, as
    /// [`client::router::Router`](crate::wire::client::router::Router)'s.
    ///
    /// # Bounded, and why nothing here waits
    ///
    /// Every queue holds one frame ahead of its reader and the finish
    /// behind it (see [`credit`]), so filing a frame is a push that
    /// always fits: the client sends only into room this end's reader
    /// gave it. [`poll_next`](Stream::poll_next) never waits on a
    /// consumer, and a frame that finds no room broke the rule and ends
    /// the connection.
    scopes: HashMap<u32, Entry>,
    /// Everything the scopes have to say, in the order they said it.
    ///
    /// See [`Notice`] for the kinds and [`drain`](Self::drain) for how
    /// they are applied.
    ///
    /// Unbounded, because half of what rides it is sent from a
    /// destructor, where there is nothing to await on and nowhere to
    /// report a failure.
    notice_receiver: UnboundedReceiver<Notice>,
    /// The other end of it, kept to clone into every scope.
    notice_sender: UnboundedSender<Notice>,
    /// Where this end's readers post the credits they owe; a task of
    /// the session's own sends them, so that reading never waits on the
    /// write half.
    outbox: UnboundedSender<Credit>,
}

/// One open scope.
#[derive(Debug)]
struct Entry {
    /// Where the channels the CLIENT opens go, whole and in arrival
    /// order: one queue for all of them, the number in each header the
    /// client's.
    requests: Filer,
    /// The channels THIS end opened, by number, and where the client's
    /// answers on each go.
    channels: HashMap<u32, Filer>,
    /// How a channel number gets given back to the handle that mints
    /// them, when its answer finishes.
    finished: UnboundedSender<u32>,
    /// This end's room for responses in the scope.
    responses: Window,
    /// This end's room for channel requests in the scope.
    asked: Window,
    /// This end's room on each channel the client opened, by the
    /// client's number.
    answers: HashMap<u32, Window>,
    /// Where each of those windows is handed to the scope's handle.
    handed: UnboundedSender<(u32, Window)>,
}

impl Entry {
    /// Rule 7 of [`credit`], for everything under the scope.
    fn end(&self, why: Ended) {
        self.responses.end(why);
        self.asked.end(why);
        for window in self.answers.values() {
            window.end(why);
        }
    }
}

impl Session {
    /// Take a connection, however it was made.
    ///
    /// Splits it, and keeps both halves: the read half to run the loop,
    /// the write half to share out. Starts the task that sends this
    /// end's credits, which ends when nothing can owe one.
    ///
    /// It neither dials nor accepts — see [`Connection`] — so what the
    /// URL is, what the TLS story is, and what authenticated the
    /// upgrade are all settled before this is called.
    pub fn new(connection: Connection) -> Self {
        let (sink, stream) = connection.split();
        let (notice_sender, notice_receiver) = mpsc::unbounded_channel();
        let (outbox, outbox_receiver) = mpsc::unbounded_channel();
        let sink = Arc::new(Mutex::new(sink));
        tokio::spawn(pump(outbox_receiver, sink.clone()));
        Session {
            stream,
            sink,
            scopes: HashMap::new(),
            notice_receiver,
            notice_sender,
            outbox,
        }
    }

    /// Make a scope's inbox and windows, and hand them out.
    ///
    /// [`None`] for a scope that is already open. An entry whose inbox
    /// is closed is a scope whose handle is gone, and it is overwritten
    /// rather than refused: a client may reuse a scope number once that
    /// scope has ended.
    fn open_scope(&mut self, scope: u32) -> Option<ScopeHandle> {
        let occupied = self.scopes.get(&scope).is_some_and(|entry| !entry.requests.is_closed());
        if occupied {
            return None;
        }
        let (requests, channel_request_receiver) = Receiver::new(self.outbox.clone(), scope, 0, true);
        let (finished, finished_channel_receiver) = mpsc::unbounded_channel();
        let (handed, answers_receiver) = mpsc::unbounded_channel();
        let (responses, asked) = (Window::new(), Window::new());
        let entry = Entry { requests, channels: HashMap::new(), finished, responses: responses.clone(), asked: asked.clone(), answers: HashMap::new(), handed };
        if let Some(old) = self.scopes.insert(scope, entry) {
            old.end(Ended::Scope);
        }
        Some(ScopeHandle::new(
            scope,
            channel_request_receiver,
            finished_channel_receiver,
            self.notice_sender.clone(),
            self.sink.clone(),
            (responses, asked),
            answers_receiver,
            self.outbox.clone(),
        ))
    }

    /// Where the client's answer on one of this end's channels goes.
    fn channel(&self, scope: u32, channel: u32) -> Option<&Filer> {
        self.scopes.get(&scope)?.channels.get(&channel)
    }

    /// Drop one of this end's channels whose answer finished, and give
    /// its number back.
    fn close_channel(&mut self, scope: u32, channel: u32) {
        let Some(entry) = self.scopes.get_mut(&scope) else { return };
        if entry.channels.remove(&channel).is_some() {
            let _ = entry.finished.send(channel);
        }
    }

    /// Apply every notice that is waiting, in order.
    ///
    /// Once per frame, before anything is looked up, which is enough: a
    /// notice that matters to a frame was sent before the frame it
    /// matters to could have been caused.
    fn drain(&mut self) {
        while let Ok(notice) = self.notice_receiver.try_recv() {
            match notice {
                Notice::Register { scope, channel, response_filer } => {
                    if let Some(entry) = self.scopes.get_mut(&scope) {
                        entry.channels.insert(channel, response_filer);
                    }
                }
                Notice::Closed(scope, None) => {
                    let gone = self.scopes.get(&scope).is_some_and(|entry| entry.requests.is_closed());
                    if gone && let Some(entry) = self.scopes.remove(&scope) {
                        entry.end(Ended::Scope);
                    }
                }
                Notice::Closed(scope, Some(channel)) => {
                    if let Some(entry) = self.scopes.get_mut(&scope)
                        && entry.channels.get(&channel).is_some_and(Filer::is_closed)
                    {
                        entry.channels.remove(&channel);
                    }
                }
                Notice::Answered(scope, channel) => {
                    if let Some(entry) = self.scopes.get_mut(&scope) {
                        entry.answers.remove(&channel);
                    }
                }
            }
        }
    }

    /// Route one frame that is not a request or a token; `Err` if it
    /// broke rule 6 of [`credit`].
    fn route(&mut self, frame: ClientFrame<'_>, bytes: Bytes) -> Result<(), Overrun> {
        match frame {
            ClientFrame::Auth { .. } | ClientFrame::Request { .. } => Ok(()),
            // The scope's queue; its window is made and handed to the
            // handle before the request is filed, so whoever answers
            // finds it.
            ClientFrame::ChannelRequest { scope, channel, .. } => match self.scopes.get_mut(&scope) {
                Some(entry) => {
                    let window = Window::new();
                    entry.answers.insert(channel, window.clone());
                    let _ = entry.handed.send((channel, window));
                    credit::file(&entry.requests, bytes)
                }
                None => Ok(()),
            },
            ClientFrame::ChannelResponse { scope, channel, .. } => match self.channel(scope, channel) {
                Some(filer) => credit::file(filer, bytes),
                None => Ok(()),
            },
            // Filed first, because the finish is what says the answer
            // ended rather than the connection.
            ClientFrame::ChannelResponseFinish { scope, channel } => {
                let filed = match self.channel(scope, channel) {
                    Some(filer) => credit::file(filer, bytes),
                    None => Ok(()),
                };
                self.close_channel(scope, channel);
                filed
            }
            // Room on a stream this end sends. An unknown stream or kind
            // is nothing.
            ClientFrame::Credit { scope, channel, kind } => {
                if let (Some(kind), Some(entry)) = (Kind::of(kind), self.scopes.get(&scope)) {
                    match kind {
                        Kind::Requests | Kind::RequestsAbandoned => entry.asked.apply(kind),
                        Kind::Responses | Kind::ResponsesAbandoned if channel == 0 => entry.responses.apply(kind),
                        Kind::Responses | Kind::ResponsesAbandoned => {
                            if let Some(window) = entry.answers.get(&channel) {
                                window.apply(kind);
                            }
                        }
                    }
                }
                Ok(())
            }
        }
    }

    /// Rule 7 of [`credit`] for the whole connection.
    fn end(&mut self) {
        for entry in self.scopes.values() {
            entry.end(Ended::Connection);
        }
    }
}

/// The connection is gone with the session: every window under it ends.
impl Drop for Session {
    fn drop(&mut self) {
        self.end();
    }
}

impl Stream for Session {
    type Item = Received;

    /// The next token or request.
    ///
    /// Everything else that arrives is routed on the way: a channel
    /// request to its scope's queue, an answer to the channel it
    /// answers, a credit to the window it names. A frame for a scope or
    /// channel with no entry is discarded — its reader went, or never
    /// was. A frame that found no room ends the stream, and with it the
    /// connection.
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Received>> {
        // Every field is `Unpin`, so this never has to project.
        let this = self.get_mut();
        loop {
            let Some(received) = ready!(this.stream.poll_next_unpin(cx))
            else {
                this.end();
                return Poll::Ready(None);
            };
            // No frame, so nothing to route.
            let Ok(bytes) = received else { continue };
            // Decoded for its header alone; what is forwarded is
            // `bytes`, whole and untouched.
            let Ok(frame) = ClientFrame::decode(&bytes) else { continue };
            this.drain();
            match frame {
                // Yielded like a request, judged by whoever reads this
                // stream.
                ClientFrame::Auth { payload } => {
                    let payload = bytes.slice_ref(payload);
                    return Poll::Ready(Some(Received::Auth(payload)));
                }
                ClientFrame::Request { scope, payload } => {
                    if let Some(handle) = this.open_scope(scope) {
                        let payload = bytes.slice_ref(payload);
                        return Poll::Ready(Some(Received::Request(payload, handle)));
                    }
                }
                frame => {
                    if this.route(frame, bytes.clone()).is_err() {
                        this.end();
                        return Poll::Ready(None);
                    }
                }
            }
        }
    }
}

/// Send every credit this end's readers owe, until none can owe any
/// more or the socket is gone.
async fn pump(mut outbox: UnboundedReceiver<Credit>, sink: Arc<Mutex<SplitSink<Connection, Bytes>>>) {
    let mut buffer = Vec::new();
    while let Some(credit) = outbox.recv().await {
        buffer.clear();
        ServerFrame::Credit { scope: credit.scope, channel: credit.channel, kind: credit.kind as u8 }
            .encode(&mut Writer::new(&mut buffer))
            .unwrap_or_else(|error| match error {});
        if sink.lock().await.send(Bytes::copy_from_slice(&buffer)).await.is_err() {
            return;
        }
    }
}
