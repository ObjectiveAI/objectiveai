//! One channel, where routing stops.

use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::Stream;
use tokio::sync::mpsc::Receiver;

/// The far end of one channel.
///
/// The end of the tree: a channel has nothing under it, so this
/// delivers frames to one consumer rather than choosing between
/// several. It is named for its place in the tree rather than for
/// doing any routing of its own.
///
/// # Whole frames, untouched
///
/// What arrives is the wire frame exactly as it came off the socket —
/// header included, nothing sliced, nothing rewritten. Whoever routed
/// it read the header to know where it belonged and left it alone.
///
/// Which is what lets a consumer tell a
/// [`ChannelResponse`](crate::frame::server::ServerFrame::ChannelResponse)
/// from a
/// [`ChannelResponseFinish`](crate::frame::server::ServerFrame::ChannelResponseFinish).
/// The end of a channel is a frame like any other, so it arrives here
/// like any other, and a consumer that has the header can see it.
/// Stripping the header would mean inventing a second way to say the
/// same thing.
///
/// It is also why frames travel rather than decoded frames: a decoded
/// [`ClientFrame`](crate::frame::client::ClientFrame) borrows from the
/// buffer it came from, so it cannot cross a channel at all. Bytes
/// can, and they cross as a refcount bump rather than a copy.
///
/// # The numbers, and what they mean
///
/// [`scope`](Self::scope) is unambiguous — one side opens scopes, so
/// there is one space.
///
/// [`channel`](Self::channel) is not. A channel this end opened and
/// one the far end opened can share a number, in spaces that never
/// meet, and which one applies is settled by the direction a frame
/// travelled. Whoever built this knew which, and did not have to say.
///
/// What that changes is who may end it: only a responder finishes a
/// channel, so one this end opened is one this end cannot close.
#[derive(Debug)]
pub struct ChannelRouter {
    /// The scope this channel is inside.
    scope: u32,
    /// The channel these frames arrived on.
    channel: u32,
    /// Where they arrive.
    ///
    /// Bounded, and that is not a capacity: a
    /// [`Receiver<T>`](Receiver) does not carry its size in its type,
    /// so whoever calls [`channel`](tokio::sync::mpsc::channel) picks
    /// the number and nothing here learns it.
    ///
    /// What bounded settles is that there IS one, which is the half of
    /// the question with an answer. Unbounded means a consumer reading
    /// slower than the far end writes grows memory until it stops, on
    /// a protocol that streams image layers.
    ///
    /// What a full one MEANS is still unsettled — see
    /// [`connection_router`](super::connection_router). This only
    /// makes sure somebody has to decide.
    receiver: Receiver<Bytes>,
}

impl ChannelRouter {
    /// Take a channel and somewhere its frames will arrive.
    pub fn new(
        scope: u32,
        channel: u32,
        receiver: Receiver<Bytes>,
    ) -> Self {
        ChannelRouter {
            scope,
            channel,
            receiver,
        }
    }

    /// The scope this channel is inside.
    pub fn scope(&self) -> u32 {
        self.scope
    }

    /// The channel these frames arrived on.
    pub fn channel(&self) -> u32 {
        self.channel
    }
}

/// The frames, until nothing more is routed here.
///
/// Ending means the sending half was dropped. That is not the same as
/// the channel finishing: a finish is a frame and arrives as one, so a
/// consumer sees it and then sees this end. Which order they come in,
/// and whether a router drops the sender at all, is the router's.
impl Stream for ChannelRouter {
    type Item = Bytes;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Bytes>> {
        self.get_mut().receiver.poll_recv(cx)
    }
}
