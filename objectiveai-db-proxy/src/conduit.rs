//! The multiplexer: many Postgres connections over one host socket.
//!
//! CONCURRENCY IS THE WHOLE JOB. A connection pool opens several
//! sockets at once and expects them to make progress in parallel,
//! which they must do while sharing a single WebSocket. Three rules
//! keep them from serializing behind each other:
//!
//! - every hand-off is an UNBOUNDED channel send, so no stream ever
//!   waits on another stream's reader;
//! - routing NEVER awaits I/O, so one slow client cannot stall the
//!   socket every other client shares;
//! - exactly ONE writer task owns the WebSocket sink, so frames
//!   interleave at chunk granularity instead of queueing behind
//!   whichever stream reached the sink first.
//!
//! The remaining coupling is bandwidth: one socket carries everything,
//! so a large result set shares it rather than blocking its siblings.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use axum::body::Bytes;
use axum::extract::ws::Message;
use dashmap::DashMap;
use tokio::sync::{mpsc, watch};

use crate::frame::Frame;

/// The attached host relay: where to put frames bound for the
/// database, and the epoch identifying which relay that is.
#[derive(Clone)]
pub struct Host {
    pub epoch: u64,
    pub tx: mpsc::UnboundedSender<Message>,
}

struct Stream {
    /// The relay this stream was opened over. A stream cannot outlive
    /// it: when that socket dies, so does the far-end Postgres
    /// connection this stream is one half of.
    epoch: u64,
    /// The client connection's inbound pump.
    tx: mpsc::UnboundedSender<Bytes>,
}

pub struct Conduit {
    next_id: AtomicU32,
    next_epoch: AtomicU64,
    streams: DashMap<u32, Stream>,
    /// The current relay, or `None` while no host is attached. A
    /// `watch` rather than a notification so a client arriving AFTER
    /// the host attached reads the value instead of waiting forever on
    /// a signal it missed.
    host: watch::Sender<Option<Host>>,
}

impl Conduit {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            // Ids start at 1 so that 0 never names a live stream.
            next_id: AtomicU32::new(1),
            next_epoch: AtomicU64::new(1),
            streams: DashMap::new(),
            host: watch::channel(None).0,
        })
    }

    /// Wait for a relay to be attached, returning it as soon as one is.
    ///
    /// A plugin can — and on a cold start will — connect before the
    /// host has dialed in. Holding here rather than buffering is
    /// deliberate: the client's bytes stay in the kernel receive
    /// buffer, so backpressure is the TCP window and this process
    /// never grows a queue of its own.
    ///
    /// `None` only once the [`Conduit`] itself is gone, which means the
    /// process is shutting down and there is nothing left to wait for.
    pub async fn host(&self) -> Option<Host> {
        let mut rx = self.host.subscribe();
        loop {
            if let Some(host) = rx.borrow_and_update().clone() {
                return Some(host);
            }
            rx.changed().await.ok()?;
        }
    }

    /// Resolve once `epoch` is no longer the attached relay — because it
    /// detached, or because a newer one superseded it.
    ///
    /// This is what a client connection watches, and it is load-bearing
    /// rather than belt-and-braces. [`detach`](Self::detach) sweeps the
    /// streams it knows about, but a client that read the relay just
    /// before it detached can register just after, leaving a stream no
    /// sweep will ever see. Such a client is typically mid-round-trip —
    /// blocked on a response that is never coming — and on a loopback
    /// socket with no keepalive it would block forever. Watching the
    /// cell makes the *value* authoritative instead of the sweep, so the
    /// race resolves itself no matter which order the two happen in.
    pub async fn detached(&self, epoch: u64) {
        let mut rx = self.host.subscribe();
        while rx
            .borrow_and_update()
            .as_ref()
            .is_some_and(|host| host.epoch == epoch)
        {
            if rx.changed().await.is_err() {
                return;
            }
        }
    }

    /// Attach a relay, returning its epoch. Newest wins: a host that
    /// redials after a blip supersedes whatever was here.
    pub fn attach(&self, tx: mpsc::UnboundedSender<Message>) -> u64 {
        let epoch = self.next_epoch.fetch_add(1, Ordering::Relaxed);
        let _ = self.host.send(Some(Host { epoch, tx }));
        epoch
    }

    /// A relay ended.
    pub fn detach(&self, epoch: u64) {
        // Clear the cell only if this relay still occupies it — a
        // newer socket that already superseded us stays attached.
        self.host.send_if_modified(|slot| {
            if slot.as_ref().is_some_and(|host| host.epoch == epoch) {
                *slot = None;
                true
            } else {
                false
            }
        });
        // Every stream opened over this relay is now orphaned. Dropping
        // its sender ends the client's inbound pump, which shuts that
        // client's socket — which is precisely what makes a pool
        // reconnect instead of waiting on a backend that is never
        // going to answer.
        self.streams.retain(|_, stream| stream.epoch != epoch);
    }

    /// Register a client connection and mint its id.
    ///
    /// Ids are monotonic and never reused within a container's life:
    /// four billion connections is not a number a plugin container
    /// reaches, so there is no wraparound to reason about.
    pub fn open(&self, epoch: u64, tx: mpsc::UnboundedSender<Bytes>) -> u32 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.streams.insert(id, Stream { epoch, tx });
        id
    }

    pub fn close(&self, id: u32) {
        self.streams.remove(&id);
    }

    /// Route one frame that arrived from the host. Never awaits.
    pub fn route(&self, frame: Frame) {
        match frame {
            Frame::Data { id, payload } => {
                // The guard must be released before `remove`, or
                // dashmap deadlocks on its own shard lock. It is: the
                // scrutinee temporary lives to the end of the `match`
                // statement, and the `remove` is after it.
                let gone = match self.streams.get(&id) {
                    Some(stream) => stream.tx.send(payload).is_err(),
                    // A frame for a stream that already closed. The
                    // host has not seen our `Close` yet; nothing to do.
                    None => false,
                };
                if gone {
                    self.streams.remove(&id);
                }
            }
            Frame::Close { id } => {
                self.streams.remove(&id);
            }
            // `Open` is minted here and only ever travels proxy→host.
            // One arriving inbound is a peer that does not understand
            // the protocol, and names no stream we could route to.
            Frame::Open { .. } => {}
        }
    }
}
