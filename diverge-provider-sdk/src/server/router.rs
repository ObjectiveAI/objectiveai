//! The read loop: frames in, and off to whoever is waiting.

use std::collections::HashMap;

use bytes::Bytes;
use futures_util::StreamExt as _;
use futures_util::stream::SplitStream;
use tokio::sync::mpsc::{Sender, UnboundedReceiver, UnboundedSender};

use crate::connection::Connection;
use crate::frame::client::ClientFrame;

/// One connection's inbound half.
///
/// Reads frames, looks up where each belongs, and forwards it. The
/// mirror of [`client::router::Router`](crate::client::router::Router),
/// and the same wherever the two halves are the same: whole frames go
/// on untouched, header included; one hop from here to the consumer;
/// bounded senders, so a consumer that stops reading stops the
/// connection rather than losing a frame; a lookup that misses drains
/// the registration queue and tries once more.
///
/// # Two kinds of frame, and only two
///
/// **Something the client asked for.** A request, or a channel request.
/// Both go to one stream exactly as they arrived, and the header says
/// which scope and which channel. Neither can be registered for in
/// advance, because the client chose the numbers and this end is
/// hearing them for the first time.
///
/// **An answer to something this end asked for.** A channel response,
/// or its finish. Those go to whoever opened the channel, which is why
/// a [`Registration`] exists at all.
///
/// # Auth is dropped on the floor
///
/// And that is a gap, not a decision. A credential belongs to the
/// connection, this reads the connection, and there is nowhere for it
/// to go — no channel out of here carries one, so the frame is
/// discarded like a type nobody defines. Whatever ends up deciding what
/// a credential means will need a path through here, and does not have
/// one.
#[derive(Debug)]
pub struct Router {
    /// The read half of the connection.
    ///
    /// Split, because the write half belongs to whoever is writing and
    /// a [`Sink`](futures_util::Sink) needs `&mut` — see
    /// [`connection`](crate::connection) for why that is a wire
    /// requirement rather than an implementation detail.
    stream: SplitStream<Connection>,
    /// Where everything the client asks for goes.
    ///
    /// Requests and channel requests both, in arrival order, whole. One
    /// stream because they have one thing in common that decides it:
    /// the client picked the numbers, so there is nothing here that
    /// could have been told about either in advance.
    ///
    /// Bounded, and a full one stops the loop like any other. The cost
    /// of not stopping would be worse here than anywhere: a dropped
    /// request is a client waiting forever, since a stream ends at its
    /// finish frame and no finish would ever come.
    request_sender: Sender<Bytes>,
    /// The channels this end opened, by scope and channel, and what the
    /// client says back on each.
    ///
    /// Flat rather than nested by scope, because nothing here ends a
    /// scope: a scope ends when this end finishes it, which happens
    /// where the writing is and is never seen from in here. So there is
    /// no removal that would want every channel of one scope at once.
    ///
    /// An entry leaves when the client's answer on it finishes. One
    /// that never finishes stays until the connection ends — including
    /// every channel still open in a scope this end has finished, which
    /// is the one thing in here that leaks.
    channels: HashMap<(u32, u32), Sender<Bytes>>,
    /// Somewhere to say an answer is coming, before the frame that asks
    /// for it goes out.
    ///
    /// See [`Registration`].
    ///
    /// Unbounded on purpose. Registering must not block, because
    /// whoever is registering is about to write the channel request
    /// that causes the answer — and a registration that waited behind a
    /// full queue would be a request whose answer arrives before
    /// anywhere exists to put it.
    registrations: UnboundedReceiver<Registration>,
    /// Where this says a channel is finished.
    ///
    /// `(scope, channel)`, in this end's numbering, sent when the
    /// client's answer runs out. It frees the number for reuse; the
    /// receiver closing says the same thing to whoever was reading the
    /// answer.
    ///
    /// Unbounded, because it is sent from inside the loop that would
    /// otherwise have to drain it.
    closed: UnboundedSender<(u32, u32)>,
}

impl Router {
    /// Take the read half, and the three ends that connect it to the
    /// writer.
    ///
    /// The read half of a split [`Connection`], the sending end of the
    /// request stream, the receiving end of the registration channel,
    /// and the sending end of the closure channel. None of them is made
    /// here: each has another end, and the other ends belong together
    /// elsewhere.
    ///
    /// No channels. A connection starts with none open, and every entry
    /// arrives through `registrations`.
    pub fn new(
        stream: SplitStream<Connection>,
        request_sender: Sender<Bytes>,
        registrations: UnboundedReceiver<Registration>,
        closed: UnboundedSender<(u32, u32)>,
    ) -> Self {
        Router {
            stream,
            request_sender,
            channels: HashMap::new(),
            registrations,
            closed,
        }
    }

    /// Read frames until the connection ends.
    ///
    /// Returns when the stream ends, and at no other point. A transport
    /// error yields no frame, so there is nothing to route and the loop
    /// takes the next one — see
    /// [`client::router::Router::run`](crate::client::router::Router::run)
    /// for why that is the rule rather than ending here.
    ///
    /// Every consumer learns the connection is over the same way: this
    /// is dropped, and every sender in it with it.
    ///
    /// # Discarding
    ///
    /// A frame nobody is waiting for is dropped, silently, and the loop
    /// carries on. It happens four ways: a header too short to read, a
    /// type this layer does not define (including `2` and `3`, which
    /// are a server's replies and not a client's to send), an auth
    /// frame, and an answer on a channel with no entry.
    ///
    /// What a client asks for cannot miss. It is not routed by scope or
    /// channel, so there is no entry for it to fail to find — only a
    /// stream nobody is reading, which is a server with nothing serving
    /// it.
    pub async fn run(mut self) {
        while let Some(received) = self.stream.next().await {
            // No frame, so nothing to route. The stream is what ends
            // this loop, and it has not ended.
            let Ok(bytes) = received else { continue };
            // Decoded for its header alone. The payload is borrowed and
            // then ignored; what gets forwarded is `bytes`, whole and
            // untouched.
            let Ok(frame) = ClientFrame::decode(&bytes) else { continue };
            match frame {
                // Nowhere to go. See the type's documentation: this is
                // the gap, not a decision.
                ClientFrame::Auth { .. } => {}
                // Both are the client asking, and both carry numbers
                // the client chose. Nothing here could have been told
                // where to put them, so they go to the one stream that
                // does not need to have been told.
                ClientFrame::Request { .. }
                | ClientFrame::ChannelRequest { .. } => {
                    let _ = self.request_sender.send(bytes).await;
                }
                ClientFrame::ChannelResponse { scope, channel, .. } => {
                    if let Some(sender) = self.channel(scope, channel) {
                        let _ = sender.send(bytes).await;
                    }
                }
                // Forwarded first, because the finish is what says the
                // answer ended rather than the connection.
                ClientFrame::ChannelResponseFinish { scope, channel } => {
                    if let Some(sender) = self.channel(scope, channel) {
                        let _ = sender.send(bytes).await;
                    }
                    self.close(scope, channel);
                }
            }
        }
    }

    /// Find a channel, draining registrations first if it is not there.
    ///
    /// The retry is not an optimization, it is the fix for a race that
    /// is otherwise unavoidable: a registration and the channel request
    /// that causes the answer travel by different routes, and the
    /// answer can beat the registration here. Draining on a miss closes
    /// the window — anything registered before the frame was read is
    /// found on the second look.
    ///
    /// A miss after the drain is a real miss: a channel this end never
    /// opened, or one whose answer already finished.
    fn channel(&mut self, scope: u32, channel: u32) -> Option<&Sender<Bytes>> {
        if !self.channels.contains_key(&(scope, channel)) {
            self.drain();
        }
        self.channels.get(&(scope, channel))
    }

    /// Drop a channel, and say so.
    ///
    /// Only on a real removal, so that whoever is counting what it
    /// opened against what it closed is never told twice. A finish for
    /// a channel that is already gone is nothing to do.
    fn close(&mut self, scope: u32, channel: u32) {
        if self.channels.remove(&(scope, channel)).is_some() {
            let _ = self.closed.send((scope, channel));
        }
    }

    /// Take every registration that is waiting, and add what it can.
    ///
    /// Registrations arrive unbounded and this empties the queue rather
    /// than taking one, because the cost is per call and not per item:
    /// it runs on a miss, and a miss is what it is trying to stop
    /// happening again.
    ///
    /// An entry that already exists stays. Replacing it would silently
    /// redirect an answer someone else is still reading, and the two
    /// claimants cannot both be right — the channel number is this
    /// end's to choose, so a collision is this end reusing a live one.
    ///
    /// It is not reported, because a registration is not a request and
    /// there is no reply to put an answer in. What the registrant sees
    /// is a receiver that stays empty.
    fn drain(&mut self) {
        while let Ok(registration) = self.registrations.try_recv() {
            self.channels
                .entry((registration.scope, registration.channel))
                .or_insert(registration.response_sender);
        }
    }
}

/// Somewhere to put an answer, arranged before it is asked for.
///
/// Sent to a [`Router`], rather than installed by one, because the
/// party that knows an answer is coming is the party about to ask for
/// it, and that is not the router. It races the answer, which a
/// [`Router`] handles by draining the queue whenever a lookup misses.
///
/// One kind, because there is only one thing this end asks for. A scope
/// is the client's to open and a scope's traffic is the client's to
/// name, so nothing about a scope can be arranged here in advance —
/// what arrives inside one goes to the request stream along with
/// everything else the client sends.
#[derive(Debug)]
pub struct Registration {
    /// The scope the channel is inside, as the client numbered it.
    pub scope: u32,
    /// The channel, chosen by this end.
    ///
    /// Unique within its scope among the channels this end has open.
    /// The client's channels are counted separately and never collide
    /// with these, which is why the pair is the key.
    pub channel: u32,
    /// Where the client's answers on it go.
    pub response_sender: Sender<Bytes>,
}
