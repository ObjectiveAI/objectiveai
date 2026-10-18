//! The channels a proxy opens on a scope, as a stream.

use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::wire::frame::server::ServerFrame;

/// Every channel the proxy opens on one scope, as it opens it: the
/// proxy's channel number, and the ask decoded — or [`None`] for a
/// payload this end cannot read, which is a channel to finish with
/// nothing before the finish, the wire's could-not-serve.
///
/// Read off the scope's inbox. Ends when the inbox does: the router
/// has closed the scope, which is the proxy gone. A frame in the
/// inbox that is not a channel request is skipped — nothing else
/// arrives there — and the decoder is the scope's own, because each
/// scope's asks are its own type.
#[must_use = "asks that are not polled are asks nobody answers"]
#[derive(Debug)]
pub struct Asks<A> {
    receiver: UnboundedReceiver<Bytes>,
    decode: fn(&[u8]) -> Option<A>,
}

impl<A> Asks<A> {
    pub(crate) fn new(receiver: UnboundedReceiver<Bytes>, decode: fn(&[u8]) -> Option<A>) -> Self {
        Asks { receiver, decode }
    }
}

impl<A> Stream for Asks<A> {
    type Item = (u32, Option<A>);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            let Some(bytes) = ready!(self.receiver.poll_recv(cx)) else {
                return Poll::Ready(None);
            };
            if let Ok(ServerFrame::ChannelRequest { channel, payload, .. }) = ServerFrame::decode(&bytes) {
                return Poll::Ready(Some((channel, (self.decode)(payload))));
            }
        }
    }
}
