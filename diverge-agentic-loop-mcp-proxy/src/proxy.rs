//! The proxy's shared state: one connection slot, 256 channels.

use std::sync::{Arc, Mutex};

use bytes::Bytes;
use diverge_provider_sdk::encode::{Encode, Writer};
use diverge_provider_sdk::endpoints::agentic_loop::run::server::channel_request;
use diverge_provider_sdk::mcp_proxy;
use tokio::sync::{Notify, mpsc, watch};

/// How many frames may queue toward the socket before senders wait.
const OUTBOUND_CAPACITY: usize = 64;

/// The connection slot and the channel table, shared by the WebSocket
/// half and the MCP handlers.
///
/// # One connection
///
/// [`claim`](Self::claim) is the door: it succeeds for exactly one
/// holder at a time, and the [`Claim`] it returns releases the slot
/// when dropped — including the drop that happens when an upgrade
/// never completes, so a connection that failed to arrive cannot wedge
/// the slot shut.
///
/// # Exchanges block until a connection exists
///
/// [`open`](Self::open) waits on the slot. A request made before any
/// connection, or after one died, sits in that wait until the next
/// connection publishes itself — the proxy's standing rule, applied by
/// construction.
///
/// # The locks are sync, and never held across an await
///
/// Every critical section here is a read-modify-write on plain
/// state. What waits — for a connection, for a freed tag — waits on
/// [`watch`] and [`Notify`], outside any lock.
pub struct Proxy {
    /// Whether a connection currently holds the slot, and the
    /// generation of the latest claim. The generation is what makes
    /// release idempotent: a stale release cannot free a newer
    /// claim's slot.
    occupancy: Mutex<Occupancy>,
    /// The live connection's outbound queue, if any. MCP handlers
    /// wait for `Some`; the WebSocket half publishes on arrival, and
    /// release clears it.
    outbound: watch::Sender<Option<Outbound>>,
    /// Response routing, one slot per channel tag.
    routes: Mutex<Routes>,
    /// Signalled when a tag is freed, for openers waiting on a full
    /// table.
    freed: Notify,
}

/// The sending half of one WebSocket connection.
#[derive(Clone)]
pub struct Outbound {
    /// Which claim this connection is. An opener that saw its send
    /// fail waits for the slot to move past this generation before
    /// trying again.
    generation: u64,
    /// Encoded frames, in order, toward the socket's write pump.
    sender: mpsc::Sender<Vec<u8>>,
}

struct Occupancy {
    taken: bool,
    generation: u64,
}

/// One slot per possible tag. `Some` is a live channel: the sender
/// routes its responses to whoever opened it, and dropping the sender
/// is how both endings — finish and connection death — look the same
/// to the opener: the stream ends.
struct Routes {
    slots: Vec<Option<mpsc::UnboundedSender<Bytes>>>,
}

/// A claimed connection slot, released on drop.
///
/// The WebSocket half holds one for the life of its connection. Its
/// drop is the release, so every exit — a clean close, a read error,
/// an upgrade that never called back — frees the slot and kills the
/// channels that were riding the connection.
pub struct Claim {
    proxy: Arc<Proxy>,
    generation: u64,
}

impl Proxy {
    pub fn new() -> Self {
        Self {
            occupancy: Mutex::new(Occupancy {
                taken: false,
                generation: 0,
            }),
            outbound: watch::Sender::new(None),
            routes: Mutex::new(Routes {
                slots: (0..=u8::MAX as usize).map(|_| None).collect(),
            }),
            freed: Notify::new(),
        }
    }

    /// Make the outbound queue a claimed connection will publish.
    pub fn queue() -> (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) {
        mpsc::channel(OUTBOUND_CAPACITY)
    }

    /// Take the connection slot, or learn that it is taken.
    pub fn claim(self: &Arc<Self>) -> Option<Claim> {
        let mut occupancy = self.occupancy.lock().expect("occupancy poisoned");
        if occupancy.taken {
            return None;
        }
        occupancy.taken = true;
        occupancy.generation += 1;
        Some(Claim {
            proxy: Arc::clone(self),
            generation: occupancy.generation,
        })
    }

    /// Publish a claimed connection's outbound queue, unblocking every
    /// waiting opener.
    pub fn publish(&self, claim: &Claim, sender: mpsc::Sender<Vec<u8>>) {
        self.outbound.send_replace(Some(Outbound {
            generation: claim.generation,
            sender,
        }));
    }

    /// Open a channel: allocate a tag, register its route, and send
    /// the request on the live connection — waiting for one if none is
    /// live, and moving to the next if the one it tried died under it.
    ///
    /// The receiver yields the channel's responses; the stream ending
    /// is the channel ending, whether by finish or by the connection.
    ///
    /// `Err` is the one failure that is this side's own: a request
    /// whose params would not serialize.
    pub async fn open(
        &self,
        request: channel_request::Frame,
    ) -> Result<mpsc::UnboundedReceiver<Bytes>, serde_json::Error> {
        let mut watcher = self.outbound.subscribe();
        loop {
            // Wait for a connection. `wait_for` sees the current value
            // first, so a live connection is found without an edge.
            let outbound = watcher
                .wait_for(Option::is_some)
                .await
                .expect("the proxy outlives its watchers")
                .clone()
                .expect("checked Some");

            let (channel, receiver) = self.allocate().await;

            let mut frame = Vec::new();
            let encoded = mcp_proxy::container::Frame {
                channel,
                // Cloned in, because a retry after a dead connection
                // needs the request again.
                request: request.clone(),
            }
            .encode(&mut Writer::new(&mut frame));
            if let Err(error) = encoded {
                self.finish(channel);
                return Err(error);
            }

            if outbound.sender.send(frame).await.is_ok() {
                return Ok(receiver);
            }

            // The connection died between the wait and the send. Free
            // the tag and wait for the slot to move past that
            // connection before trying again.
            self.finish(channel);
            let _ = watcher
                .wait_for(|slot| match slot {
                    Some(current) => current.generation != outbound.generation,
                    None => true,
                })
                .await;
        }
    }

    /// Route one response to its channel's opener. A response on a
    /// dead tag is dropped — the opener is gone, and there is nobody
    /// to tell.
    pub fn respond(&self, channel: u8, payload: Bytes) {
        let routes = self.routes.lock().expect("routes poisoned");
        if let Some(sender) = &routes.slots[channel as usize] {
            let _ = sender.send(payload);
        }
    }

    /// End a channel: drop its route — the opener sees the stream
    /// end — and return the tag to the pool.
    pub fn finish(&self, channel: u8) {
        let mut routes = self.routes.lock().expect("routes poisoned");
        routes.slots[channel as usize] = None;
        drop(routes);
        self.freed.notify_waiters();
    }

    /// Allocate the lowest free tag, waiting if all 256 are live.
    async fn allocate(&self) -> (u8, mpsc::UnboundedReceiver<Bytes>) {
        loop {
            let notified = std::pin::pin!(self.freed.notified());
            {
                let mut routes = self.routes.lock().expect("routes poisoned");
                if let Some(index) =
                    routes.slots.iter().position(Option::is_none)
                {
                    let (sender, receiver) = mpsc::unbounded_channel();
                    routes.slots[index] = Some(sender);
                    return (index as u8, receiver);
                }
            }
            notified.await;
        }
    }

    /// The release behind [`Claim`]'s drop: clear the slot if this
    /// claim still holds it, and kill every live channel — their
    /// exchanges were not served, and their openers see the stream
    /// end. A no-op for a stale claim.
    fn release(&self, generation: u64) {
        {
            let mut occupancy =
                self.occupancy.lock().expect("occupancy poisoned");
            if occupancy.generation != generation {
                return;
            }
            occupancy.taken = false;
        }
        self.outbound.send_if_modified(|slot| match slot {
            Some(outbound) if outbound.generation == generation => {
                *slot = None;
                true
            }
            _ => false,
        });
        {
            let mut routes = self.routes.lock().expect("routes poisoned");
            for slot in routes.slots.iter_mut() {
                *slot = None;
            }
        }
        self.freed.notify_waiters();
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        self.proxy.release(self.generation);
    }
}
