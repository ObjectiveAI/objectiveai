//! One scope, and everything a provider does inside it.

use std::collections::HashSet;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::SinkExt as _;
use futures_util::stream::SplitSink;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::session::Notice;
use crate::connection::Connection;
use crate::encode::{Encode, Writer};
use crate::frame::server::ServerFrame;

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
/// [`Session`](super::session::Session) forgets the scope — and every
/// channel opened inside it — on its next drain. It is a destructor
/// rather than a method because that is what makes it unforgettable:
/// abandoning a scope and ending one deliberately are the same event to
/// the session, and neither can be omitted by a caller who did not read
/// this.
///
/// What the CLIENT sees is another matter, and it is nothing at all: *a
/// stream ends at its finish frame, and nowhere else*, so a scope
/// dropped without one leaves the client's reader waiting until the
/// socket dies. Nothing anywhere will time it out.
///
/// # What is not here yet
///
/// **Reading.** The request that opened the scope is held and not
/// exposed, and the channel requests the client opens arrive in a queue
/// with no way to take them off it. So a provider can say things but
/// cannot yet find out what it was asked.
///
/// **Answering a channel the client opened.** Which is the other half
/// of that queue, and waits on it.
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
    // The one field with no reader yet. The attribute goes when
    // something exposes the request.
    #[allow(dead_code)]
    request: Bytes,
    /// The channels the client opens inside this scope.
    ///
    /// Whole frames, and the channel number in each header is the
    /// CLIENT's — it is what an answer has to quote to be understood.
    ///
    /// Unbounded, so that a [`Session`](super::session::Session) can
    /// deliver into it without waiting — which is what lets a session
    /// be a [`Stream`](futures_util::Stream) at all. The obligation
    /// that comes back the other way is to READ it.
    ///
    /// It is also how a [`Session`](super::session::Session) tells a
    /// live scope from an ended one. Closing this closes the sender it
    /// kept, and that — not the notice beside it — is what the session
    /// actually checks.
    channel_request_receiver: UnboundedReceiver<Bytes>,
    /// Where the next channel number in this scope comes from.
    ///
    /// It starts at `0` and pre-increments, so the first channel is
    /// `1`. Channel `0` is the scope's own stream and is never minted.
    ///
    /// It wraps at the top, which is why it is not enough on its own:
    /// after a wrap the numbers below it may still be in use, and only
    /// [`channels`](Self::channels) knows.
    counter: u32,
    /// Which channel numbers in this scope are spoken for.
    ///
    /// Not the session's map, though it holds the same numbers. That
    /// one says where a frame GOES; this one says what has been handed
    /// out and not yet taken back, which is the question a minter asks
    /// and a router never does. The same split
    /// [`client::handle::Handle`](crate::client::handle::Handle) makes
    /// against its own router.
    ///
    /// A set, because there is nothing to store against a number. What
    /// arrives on a channel goes to a receiver somebody else holds; the
    /// only fact kept here is that the number is in use.
    ///
    /// An entry leaves when
    /// [`finished_channels`](Self::finished_channels) says so, and only
    /// then.
    channels: HashSet<u32>,
    /// Where the session says a channel number is free again.
    ///
    /// It arrives after the finish frame it followed, and it is not
    /// that frame's duplicate: the finish tells whoever was reading
    /// that answer that the answer is over, and this tells the minter
    /// that the number can be used again.
    ///
    /// # Why a finish and not a drop
    ///
    /// Because a finish is the client saying it will send nothing more
    /// under that number, which is the only thing that makes reuse
    /// safe. A [`Channel`] being dropped says a consumer walked away —
    /// which the client was never told, so it may still be sending, and
    /// a number freed on that signal could be minted again and route
    /// the old channel's late frames into the new one.
    ///
    /// So a channel abandoned without a finish keeps its number until
    /// the connection ends. That is a leak, it is bounded by what this
    /// scope abandoned, and it is the same one
    /// [`client::router::Router`](crate::client::router::Router)
    /// documents.
    ///
    /// Unbounded, because it is sent from inside the loop that would
    /// otherwise have to drain it.
    finished_channels: UnboundedReceiver<u32>,
    /// What this scope tells the session: where an answer goes, and
    /// what is over.
    ///
    /// Unbounded, because half of what rides it is sent from a
    /// destructor, and the other half must not block — a registration
    /// that waited would be a channel request whose answer arrives
    /// before anywhere exists to put it.
    notices: UnboundedSender<Notice>,
    /// The write half of the connection, shared with every other scope
    /// on it.
    ///
    /// Locked for one frame and never for a stream of them. One frame
    /// is the wire's unit of exclusion, and a scope holding this across
    /// a stream of chunks is a scope that has taken the connection away
    /// from everybody else until it finishes.
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
    /// The bookkeeping is not among them. A scope starts having opened
    /// nothing and written nothing, its first channel at `1`, and there
    /// is no other answer a caller could give.
    pub(super) fn new(
        scope: u32,
        request: Bytes,
        channel_request_receiver: UnboundedReceiver<Bytes>,
        finished_channels: UnboundedReceiver<u32>,
        notices: UnboundedSender<Notice>,
        sink: Arc<Mutex<SplitSink<Connection, Bytes>>>,
    ) -> Self {
        ScopeHandle {
            scope,
            request,
            channel_request_receiver,
            counter: 0,
            channels: HashSet::new(),
            finished_channels,
            notices,
            sink,
            buffer: Vec::new(),
        }
    }

    /// Answer, on the scope's own stream.
    ///
    /// Any number of these, then exactly one
    /// [`send_response_finish`](Self::send_response_finish). Nothing
    /// else ends the answer: a slow one is not a finished one, and
    /// there is no timeout anywhere in this protocol.
    ///
    /// The payload is written as given, tag and all. This layer does
    /// not know what an answer says; see
    /// [`endpoints`](crate::endpoints) for who does.
    ///
    /// # The scope is never an argument
    ///
    /// It is a field, so it cannot be wrong. The client's equivalent
    /// has to be told which scope it is answering in, because one
    /// handle serves every scope on the connection; here the handle IS
    /// the scope, and a misdirected response is not expressible.
    pub async fn send_response(&mut self, payload: &[u8]) {
        self.send_frame(ServerFrame::Response {
            scope: self.scope,
            payload,
        })
        .await;
    }

    /// End the scope.
    ///
    /// One frame, no payload, and everything belonging to this scope is
    /// over — the answer, and every channel either side opened inside
    /// it. The client learns it here and nowhere else.
    ///
    /// Any [`Channel`] still held from this scope will see its stream
    /// close without a finish frame, which is indistinguishable from
    /// losing the connection. That is correct: the scope it belonged to
    /// is gone, and so is whatever it was answering.
    ///
    /// # It consumes the handle
    ///
    /// Which makes *any number of responses, then exactly one finish,
    /// then nothing* a property of the type rather than a line in a
    /// document. There is no state in which a finished scope can be
    /// answered again, because there is no handle left to answer with.
    ///
    /// [`client::handle::Handle`](crate::client::handle::Handle) cannot
    /// do this and does not try: a client does not decide when its
    /// scope ends, it finds out.
    ///
    /// # The inbox closes first, and this is where it matters most
    ///
    /// A client may reuse a scope number the instant it reads this
    /// finish. If the number still looked live when the reused
    /// [`Request`](crate::frame::client::ClientFrame::Request) arrived,
    /// a [`Session`](super::session::Session) would take it for a
    /// duplicate and discard it — a legitimate request dropped in
    /// silence, and a client hanging on a scope that was never
    /// answered.
    ///
    /// Leaving it to the drop is not enough. The drop runs after this
    /// returns, and what it races is a round trip that has already
    /// begun. Closing before the frame goes out means the number reads
    /// as free from the moment the client could possibly act on it.
    pub async fn send_response_finish(mut self) {
        self.channel_request_receiver.close();
        self.send_frame(ServerFrame::ResponseFinish { scope: self.scope })
            .await;
    }

    /// Open a channel inside this scope, and send the request that
    /// opens it.
    ///
    /// For the things a provider needs from a caller mid-scope: serving
    /// an image, reaching a database, running a command. Comes back
    /// with the number and the stream the client's answer will arrive
    /// on — see [`Channel`].
    ///
    /// The payload is written as given, tag and all. This layer does
    /// not know what a channel request says; see
    /// [`endpoints`](crate::endpoints) for who does.
    ///
    /// # It cannot fail
    ///
    /// Where the client's returns [`Option`] because the scope might
    /// have ended underneath it, holding one of these IS the scope
    /// being open. Minting cannot fail, and encoding a frame never
    /// could.
    ///
    /// A write that fails is dropped. The receiver in the returned
    /// [`Channel`] closes without a finish frame, which is what a
    /// caller reads as "this channel is not happening".
    ///
    /// # The registration goes first
    ///
    /// Before the frame reaches the socket, so it is always in the
    /// queue before the client could possibly answer — which is what
    /// makes a session's drain-on-miss enough to close the race rather
    /// than merely narrow it.
    pub async fn send_channel_request(&mut self, payload: &[u8]) -> Channel {
        self.take_back();
        let channel = self.mint_channel();
        let (response_sender, responses) = mpsc::unbounded_channel();
        let _ = self.notices.send(Notice::Register {
            scope: self.scope,
            channel,
            response_sender,
        });
        self.send_frame(ServerFrame::ChannelRequest {
            scope: self.scope,
            channel,
            payload,
        })
        .await;
        Channel {
            channel,
            responses,
            scope: self.scope,
            notices: self.notices.clone(),
        }
    }

    /// Take back every channel number the session says is free.
    ///
    /// The queue is emptied rather than sampled, and it is emptied
    /// before minting, because what it holds is exactly the numbers
    /// that are available again. A mint that ran first would step over
    /// them and hand out a larger number for no reason.
    fn take_back(&mut self) {
        while let Ok(channel) = self.finished_channels.try_recv() {
            self.channels.remove(&channel);
        }
    }

    /// Take the next free channel number, and claim it.
    ///
    /// Counts up and steps over anything open, wrapping at the top
    /// rather than stopping there. Which is why the counter is not
    /// enough on its own: after a wrap the numbers below it may still
    /// be in use, and only [`channels`](Self::channels) knows.
    ///
    /// The claim and the question are one move — a
    /// [`HashSet`] insert answers "was it free" and takes it at the
    /// same time — which also means a number minted is a number
    /// recorded, and the next call cannot hand out the same one.
    ///
    /// This is
    /// [`client::handle::Handle`](crate::client::handle::Handle)'s
    /// channel minting, one side over. It never returns if all four
    /// billion are open, which is not a case worth handling: that is a
    /// single scope holding four billion unfinished channels, and the
    /// set itself would not fit in memory first.
    fn mint_channel(&mut self) -> u32 {
        loop {
            self.counter = self.counter.wrapping_add(1);
            if self.channels.insert(self.counter) {
                return self.counter;
            }
        }
    }

    /// Build one frame and write it.
    ///
    /// Every frame this scope sends goes through here, which is what
    /// keeps the buffer's protocol in one place: clear, encode, copy
    /// out, write. Cleared and not merely reused because a [`Writer`]
    /// appends from wherever the buffer already ends, so an uncleared
    /// one would send the last frame with this one glued to its back.
    ///
    /// The lock is taken after the encoding and released with the
    /// statement, so it covers one write and nothing else.
    ///
    /// A failed write is dropped. It means the connection is gone, and
    /// everything on it is about to find out on its own — there is
    /// nothing this could tell a caller that the caller is not about to
    /// learn.
    ///
    /// There is no failed encode. A frame is a header and a payload
    /// this crate never looks at, so encoding one is
    /// [`Infallible`](std::convert::Infallible) and says so in the
    /// type.
    async fn send_frame(&mut self, frame: ServerFrame<'_>) {
        self.buffer.clear();
        frame
            .encode(&mut Writer::new(&mut self.buffer))
            .unwrap_or_else(|error| match error {});
        let bytes = Bytes::copy_from_slice(&self.buffer);
        let _ = self.sink.lock().await.send(bytes).await;
    }
}

/// Tell the session the scope is over.
///
/// # The receiver closes first
///
/// And the order is what makes the notice safe to act on. A
/// [`Session`](super::session::Session) does not remove a scope because
/// it was told to — it removes one whose inbox reads as closed, since
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
        let _ = self.notices.send(Notice::Closed(self.scope, None));
    }
}

/// A channel this end opened inside a scope, and what comes back on it.
///
/// What [`ScopeHandle::send_channel_request`] gives back. The request
/// has gone out, and the session is holding the other end of
/// [`responses`](Self::responses).
///
/// The number is this end's. The client counts its own channels
/// separately and from zero, so a client's channel `1` and this one are
/// unrelated — which is why they never share a stream.
///
/// # Read it or drop it
///
/// [`responses`](Self::responses) is unbounded, and what rides it is a
/// stream rather than a message — an image layer, a database
/// connection, a command's items. An answer nobody reads is memory the
/// far side can grow without limit, and nothing in this crate bounds
/// it. Dropping this frees the queue and tells the session to forget
/// the channel.
#[derive(Debug)]
pub struct Channel {
    /// The channel's number, chosen by this end.
    ///
    /// Meaningful only inside the scope it was opened in.
    pub channel: u32,
    /// The client's answer, frame by frame.
    ///
    /// Whole frames, headers included. Ends at the finish frame; the
    /// channel closing without one means the connection went first, or
    /// the scope did.
    pub responses: UnboundedReceiver<Bytes>,
    /// The scope it belongs to, for the notice at the end.
    ///
    /// Not public, because it is not this type's to tell — a caller
    /// that wants the scope's number has the
    /// [`ScopeHandle`] it came from.
    scope: u32,
    /// Where to say this channel is over.
    notices: UnboundedSender<Notice>,
}

/// Tell the session the channel is over.
///
/// The same shape as [`ScopeHandle`]'s, one level down and for the same
/// reason: close first, then say so, so that the session's check reads
/// as closed the moment the notice is visible.
///
/// A channel whose answer finished has already been forgotten — the
/// session drops the entry when it forwards the finish frame — so this
/// is for the other case, a caller that walked away mid-answer.
impl Drop for Channel {
    fn drop(&mut self) {
        self.responses.close();
        let _ = self
            .notices
            .send(Notice::Closed(self.scope, Some(self.channel)));
    }
}
