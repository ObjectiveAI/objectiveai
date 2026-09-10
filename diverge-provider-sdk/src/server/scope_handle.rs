//! One scope, and everything a provider does inside it.

use std::collections::HashSet;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::SinkExt as _;
use futures_util::stream::SplitSink;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::channel::Channel;
use super::notice::Notice;
use crate::connection::Connection;
use crate::encode::{Encode, Writer};
use crate::frame::server::ServerFrame;

/// A scope a client opened, and the means to answer it.
///
/// What a [`Session`](super::session::Session) yields, beside the
/// request that opened it. The scope is open from the moment this
/// exists — the request has arrived, and the session already knows
/// where to put what follows it.
///
/// This is the half that writes. A [`Session`](super::session::Session)
/// reads a socket and hands out scopes; everything a provider actually
/// SAYS is said through one of these, which is why it and not the
/// session carries the connection's write half.
///
/// # Shareable, and not [`Clone`]
///
/// Every method here takes `&self`, so an [`Arc`] of one is enough for
/// several tasks to answer the same scope at once. Which is what a
/// two-directional endpoint
/// needs: an
/// [`agent container`](crate::endpoints::containers::agents) relays
/// chunks down while relaying tool calls out, and a handle that had to be held
/// exclusively would have put a queue and an arbiter between those two
/// jobs for no reason but the signature.
///
/// The state that makes that safe is guarded rather than exclusive:
/// the socket was already behind a lock, and the channel minter and the
/// encode buffer are now. None of it is contended in practice — each is
/// held for one frame, never across a stream of them.
///
/// It is still not [`Clone`], because two things here are genuinely
/// singular. It holds a receiver, and a receiver has one reader. And
/// ending the scope consumes the handle, which is what makes one finish
/// per scope a fact rather than a rule — a share cannot reach it, and
/// something has to prove the shares are gone before it can.
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
/// # The request is not in here
///
/// It travels BESIDE this, as the other half of what a session yields:
/// a request is read once, by whatever dispatches on it, and a handle
/// that carried the bytes too would be a second copy of a thing with
/// one reader. So this holds every frame a provider can send and the
/// one thing it can receive —
/// [`recv_channel_request`](Self::recv_channel_request), what the
/// client opens inside the scope, which is where the two
/// `channel_response` calls get the channel number they have to
/// quote.
#[derive(Debug)]
pub struct ScopeHandle {
    /// The scope's number, chosen by the CLIENT.
    ///
    /// It is in the header of every frame belonging to this scope, in
    /// both directions. What it is free for afterwards is the client's
    /// business; this end never mints one and never reuses one.
    scope: u32,
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
    ///
    /// Behind a lock because everything else on this type is: a handle
    /// that can be shared has to be shareable whole. Only one task ever
    /// reads it, so the lock is never contended — it is what makes
    /// [`recv_channel_request`](Self::recv_channel_request) take
    /// `&self` like the rest, not a way of arbitrating anything.
    channel_request_receiver: Mutex<UnboundedReceiver<Bytes>>,
    /// Everything it takes to hand out a channel number.
    ///
    /// One lock rather than three fields, because minting reads all of
    /// them and the read has to be atomic: taking numbers back and then
    /// choosing one are a single decision, and two tasks interleaving
    /// halfway through it could hand out the same number twice.
    ///
    /// The type is private, so what it holds is described there and
    /// not here.
    minter: Mutex<Minter>,
    /// What this scope tells the session: where an answer goes, and
    /// what is over.
    ///
    /// Unbounded, because half of what rides it is sent from a
    /// destructor, and the other half must not block — a registration
    /// that waited would be a channel request whose answer arrives
    /// before anywhere exists to put it.
    notice_sender: UnboundedSender<Notice>,
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
    ///
    /// Guarded, by the same lock everything else here uses. Tokio's and
    /// not the standard library's: one kind of lock in an async type is
    /// one fewer thing to get wrong, and its guard is [`Send`], so
    /// nothing written here can accidentally produce a future that is
    /// not.
    ///
    /// It is still held for the encoding alone. The socket's lock is
    /// taken afterwards and separately, which is the whole point of the
    /// buffer being per scope.
    buffer: Mutex<Vec<u8>>,
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
        channel_request_receiver: UnboundedReceiver<Bytes>,
        finished_channel_receiver: UnboundedReceiver<u32>,
        notice_sender: UnboundedSender<Notice>,
        sink: Arc<Mutex<SplitSink<Connection, Bytes>>>,
    ) -> Self {
        ScopeHandle {
            scope,
            channel_request_receiver: Mutex::new(channel_request_receiver),
            minter: Mutex::new(Minter {
                counter: 0,
                channels: HashSet::new(),
                finished_channel_receiver,
            }),
            notice_sender,
            sink,
            buffer: Mutex::new(Vec::new()),
        }
    }

    /// Take the next channel the client opened inside this scope.
    ///
    /// [`None`] when the client will open no more — it left, or the
    /// connection did. A scope whose client is gone can still be
    /// answered into the void, so this is the earliest a provider
    /// learns to stop working.
    ///
    /// # Whole frames
    ///
    /// Header included, because a
    /// caller needs what is in it: the channel number is the CLIENT's,
    /// and it is what
    /// [`send_channel_response`](Self::send_channel_response) has to
    /// quote to be understood. Slicing it off here would throw away the
    /// one thing that makes an answer addressable.
    ///
    /// # It is cancel-safe
    ///
    /// Nothing is taken off the queue by a poll that does not complete,
    /// so this can lose a `select` and be called again without dropping
    /// a channel request. Which is what a stream-shaped scope needs — a
    /// [`watch`](crate::endpoints::volumes::watch) races this against
    /// the tree it is reporting, and one of the two loses every time
    /// round.
    pub async fn recv_channel_request(&self) -> Option<Bytes> {
        self.channel_request_receiver.lock().await.recv().await
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
    pub async fn send_response(&self, payload: &[u8]) {
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
    /// # Once, and last
    ///
    /// *Any number of responses, then exactly one finish, then
    /// nothing.* It used to consume the handle, which made that a
    /// property of the type; it takes `&self` now, because a container
    /// scope is held behind an [`Arc`] by every task that serves it —
    /// the relay, the channels, the directory a connector looks it up
    /// in — and a finish that needed the last reference would wait on
    /// all of them. So the rule is the handler's to keep, and every
    /// handler keeps it the same way: the finish is the last thing it
    /// does, after everything it started has been joined or told.
    ///
    /// Not while another task is inside
    /// [`recv_channel_request`](Self::recv_channel_request): that holds
    /// the inbox, and this closes it. A handler's serve loop has
    /// returned before it finishes.
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
    pub async fn send_response_finish(&self) {
        self.channel_request_receiver.lock().await.close();
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
    pub async fn send_channel_request(&self, payload: &[u8]) -> Channel {
        let channel = self.mint().await;
        let (response_sender, response_receiver) = mpsc::unbounded_channel();
        let _ = self.notice_sender.send(Notice::Register {
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
            response_receiver,
            scope: self.scope,
            notice_sender: self.notice_sender.clone(),
        }
    }

    /// Answer, on a channel the client opened.
    ///
    /// Any number of these, including none, and then exactly one
    /// [`send_channel_response_finish`](Self::send_channel_response_finish).
    ///
    /// # The channel is quoted, never chosen
    ///
    /// It comes out of the header of a channel request the client sent,
    /// and it is in the CLIENT's numbering — so it is an argument here
    /// where [`send_response`](Self::send_response) needs none. The
    /// scope is still a field, because that one is the same in both
    /// directions.
    ///
    /// Which is why nothing here touches the mint set. A client's
    /// channel `5` and this scope's channel `5` are different channels,
    /// told apart by which way a frame travelled, and this end tracks
    /// only the numbers it hands out. Claiming a quoted number would
    /// take one out of circulation for no reason; freeing one would
    /// free somebody else's.
    ///
    /// # Nothing is checked
    ///
    /// For the same reason. The numbers are the client's, so this end
    /// has nothing to check against — a misquoted channel is a frame
    /// the client discards, and this end never hears about it.
    pub async fn send_channel_response(
        &self,
        channel: u32,
        payload: &[u8],
    ) {
        self.send_frame(ServerFrame::ChannelResponse {
            scope: self.scope,
            channel,
            payload,
        })
        .await;
    }

    /// End an answer on a channel the client opened.
    ///
    /// One frame, no payload, and the channel is over. The far side
    /// learns the answer is complete here and nowhere else — the same
    /// rule this end relies on when it reads a [`Channel`].
    ///
    /// Send it even for an answer that carried nothing. An empty answer
    /// and an answer still coming are the same thing until this
    /// arrives.
    ///
    /// It frees nothing on this side, because it took nothing: the
    /// number was the client's, and giving it back is the client's to
    /// do. Compare
    /// [`Session`](super::session::Session), which hands a number back
    /// to this handle when the client finishes a channel THIS end
    /// opened.
    pub async fn send_channel_response_finish(&self, channel: u32) {
        self.send_frame(ServerFrame::ChannelResponseFinish {
            scope: self.scope,
            channel,
        })
        .await;
    }

    /// Take the next free channel number, and claim it.
    ///
    /// The minting itself is on the private type this locks; this is
    /// the lock, and the lock is why taking numbers back and choosing
    /// one cannot be separate methods any more. A task that took them
    /// back and then released the lock before choosing would let
    /// another task choose the same number in between.
    async fn mint(&self) -> u32 {
        self.minter.lock().await.mint()
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
    async fn send_frame(&self, frame: ServerFrame<'_>) {
        // The guard is released with the block, before the socket's
        // is taken. Two locks, never nested.
        let bytes = {
            let mut buffer = self.buffer.lock().await;
            buffer.clear();
            frame
                .encode(&mut Writer::new(&mut buffer))
                .unwrap_or_else(|error| match error {});
            Bytes::copy_from_slice(&buffer)
        };
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
        self.channel_request_receiver.get_mut().close();
        let _ = self.notice_sender.send(Notice::Closed(self.scope, None));
    }
}

/// Everything it takes to hand out a channel number in one scope.
///
/// Three fields that are never read apart, which is why they are one
/// thing: minting reads all of them and the read has to be atomic. Two
/// tasks interleaving halfway through would hand out the same number
/// twice, and only this end could have prevented it.
#[derive(Debug)]
struct Minter {
    /// Where the next channel number comes from.
    ///
    /// It starts at `0` and pre-increments, so the first channel is
    /// `1`. Channel `0` is the scope's own stream and is never minted.
    ///
    /// It wraps at the top, which is why it is not enough on its own:
    /// after a wrap the numbers below it may still be in use, and only
    /// [`channels`](Self::channels) knows.
    counter: u32,
    /// Which channel numbers are spoken for.
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
    finished_channel_receiver: UnboundedReceiver<u32>,
}

impl Minter {
    /// Take the next free channel number, and claim it.
    ///
    /// Counts up and steps over anything open, wrapping at the top
    /// rather than stopping there.
    ///
    /// # The queue is emptied first
    ///
    /// Rather than sampled, and before minting rather than after,
    /// because what it holds is exactly the numbers that are available
    /// again. A mint that ran first would step over them and hand out a
    /// larger number for no reason.
    ///
    /// # The claim and the question are one move
    ///
    /// A [`HashSet`] insert answers "was it free" and takes it at the
    /// same time — which also means a number minted is a number
    /// recorded, and the next call cannot hand out the same one.
    ///
    /// This is
    /// [`client::handle::Handle`](crate::client::handle::Handle)'s
    /// channel minting, one side over. It never returns if all four
    /// billion are open, which is not a case worth handling: that is a
    /// single scope holding four billion unfinished channels, and the
    /// set itself would not fit in memory first.
    fn mint(&mut self) -> u32 {
        while let Ok(channel) = self.finished_channel_receiver.try_recv() {
            self.channels.remove(&channel);
        }
        loop {
            self.counter = self.counter.wrapping_add(1);
            if self.channels.insert(self.counter) {
                return self.counter;
            }
        }
    }
}
