//! The proxy's shared state: one connection slot, 256 channels, and
//! the asking that outlives connections.

use std::sync::Arc;

use bytes::Bytes;
use diverge_provider_sdk::encode::{Encode, Writer};
use diverge_provider_sdk::endpoints::agentic_loop::run::server::channel_request;
use diverge_provider_sdk::mcp_proxy;
use tokio::sync::{Mutex, Notify, mpsc, watch};

/// How many frames may queue toward the socket before senders wait.
const OUTBOUND_CAPACITY: usize = 64;

/// What arrives on a channel an opener holds.
///
/// The third thing that can happen — the connection dying — is not a
/// variant, because nothing sends it: the route's sender is dropped
/// without a [`Finished`](ChannelEvent::Finished), and the opener
/// reads the stream ending un-finished as the death it is.
#[derive(Debug, Clone)]
pub enum ChannelEvent {
    /// One response frame's payload.
    Response(Bytes),
    /// The server's finish: the channel is over, deliberately.
    Finished,
}

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
/// # An exchange is answered, or it is asked again
///
/// [`ask`](Self::ask) does not fail because a connection did. An
/// exchange counts as answered only when its response has arrived AND
/// its channel has finished; a connection dying before that point
/// un-answers it, and the ask is sent again on the next connection —
/// forever, because the agent is waiting and nothing here has the
/// standing to tell it no. The far side may therefore see the same
/// logical ask as two wire exchanges, and a re-sent `tools/call` may
/// execute twice; that is the chosen trade.
///
/// # The routes belong to a generation
///
/// The channel table is stamped with the generation of the connection
/// it serves. Registration checks the stamp under the same lock, and
/// release wipes only its own generation's table — so a route can
/// neither be registered into a table that was already wiped (a
/// channel nothing would ever end) nor wiped out of a newer
/// connection's table (a tag freed while the far side still holds it).
///
/// # The locks are short, and never held across the wire
///
/// Every critical section here is a read-modify-write on plain
/// state. What waits — for a connection, for a freed tag, for a
/// frame — waits on [`watch`], [`Notify`] and the routes' queues,
/// outside any lock.
pub struct Proxy {
    /// Whether a connection currently holds the slot, and the
    /// generation of the latest claim. The generation is what makes
    /// release idempotent: a stale release cannot free a newer
    /// claim's slot.
    occupancy: Mutex<Occupancy>,
    /// The live connection's outbound queue, if any. Askers wait for
    /// `Some`; the WebSocket half publishes on arrival, and release
    /// clears it.
    outbound: watch::Sender<Option<Outbound>>,
    /// Response routing, one slot per channel tag, stamped with the
    /// generation it serves.
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

/// One slot per possible tag, and the generation the table serves.
///
/// `Some` is a live channel: the sender routes its events to whoever
/// opened it. [`Proxy::finish`] sends [`ChannelEvent::Finished`]
/// before freeing a slot; the wipe in release frees without it —
/// which is exactly the difference an opener reads.
struct Routes {
    slots: Vec<Option<mpsc::UnboundedSender<ChannelEvent>>>,
    /// The generation whose connection these routes ride, or `0` for
    /// none. Registration and wipe both check it under the lock.
    current: u64,
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

/// How one wire attempt of an ask ended.
enum Attempt {
    /// A response arrived and the finish followed it.
    Answered(Bytes),
    /// The finish arrived with no response before it: the far side's
    /// deliberate "could not be served."
    Refused,
    /// The stream ended un-finished: the connection died. The ask is
    /// still owed an answer.
    Died,
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
                current: 0,
            }),
            freed: Notify::new(),
        }
    }

    /// Make the outbound queue a claimed connection will publish.
    pub fn queue() -> (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) {
        mpsc::channel(OUTBOUND_CAPACITY)
    }

    /// Take the connection slot, or learn that it is taken.
    pub async fn claim(self: &Arc<Self>) -> Option<Claim> {
        let mut occupancy = self.occupancy.lock().await;
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
    ///
    /// The routes are stamped BEFORE the queue is visible, so an
    /// opener that sees the connection can always register on it.
    pub async fn publish(&self, claim: &Claim, sender: mpsc::Sender<Vec<u8>>) {
        {
            let mut routes = self.routes.lock().await;
            routes.current = claim.generation;
        }
        self.outbound.send_replace(Some(Outbound {
            generation: claim.generation,
            sender,
        }));
    }

    /// Ask, until answered.
    ///
    /// One wire exchange per live connection, re-sent on the next
    /// connection every time one dies before the answer finished.
    /// Returns the response bytes, or `None` for the far side's
    /// deliberate empty finish — the one non-answer that is not
    /// retried, because it is a statement rather than an accident.
    ///
    /// `Err` is the one failure that is this side's own: a request
    /// whose params would not serialize. It is checked on the first
    /// attempt and cannot appear later — the request does not change
    /// between attempts.
    pub async fn ask(
        &self,
        request: channel_request::Frame,
    ) -> Result<Option<Bytes>, serde_json::Error> {
        loop {
            let (_, mut receiver) = self.open(request.clone()).await?;

            let mut response = None;
            let attempt = loop {
                match receiver.recv().await {
                    // The first response is the answer; a unary
                    // channel has no business carrying a second, and
                    // extras are ignored rather than obeyed.
                    Some(ChannelEvent::Response(bytes)) => {
                        response.get_or_insert(bytes);
                    }
                    Some(ChannelEvent::Finished) => {
                        break match response.take() {
                            Some(bytes) => Attempt::Answered(bytes),
                            None => Attempt::Refused,
                        };
                    }
                    None => break Attempt::Died,
                }
            };

            match attempt {
                Attempt::Answered(bytes) => return Ok(Some(bytes)),
                Attempt::Refused => return Ok(None),
                // A response without its finish died with the
                // connection: un-answered, and asked again.
                Attempt::Died => continue,
            }
        }
    }

    /// Open a channel once: allocate a tag on the live connection and
    /// send the request — waiting for a connection if none is live,
    /// and moving to the next if the one it tried died under it.
    ///
    /// The receiver yields the channel's [`ChannelEvent`]s; the
    /// stream ending without a `Finished` is the connection dying.
    /// Alongside it comes the generation of the connection the
    /// channel was sent on, for a caller that wants to outwait that
    /// connection — see [`wait_past`](Self::wait_past).
    ///
    /// [`ask`](Self::ask) is this plus the retry law, and is what the
    /// unary exchanges use; the resident notifications stream opens
    /// directly, because it consumes the stream rather than an
    /// answer.
    pub async fn open(
        &self,
        request: channel_request::Frame,
    ) -> Result<(u64, mpsc::UnboundedReceiver<ChannelEvent>), serde_json::Error>
    {
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

            // Allocation is refused when the table no longer belongs
            // to this connection — the release already ran — in which
            // case the next connection is waited for like any other.
            let Some((channel, receiver)) =
                self.allocate(outbound.generation).await
            else {
                self.next(&mut watcher, outbound.generation).await;
                continue;
            };

            let mut frame = Vec::new();
            let encoded = mcp_proxy::container::Frame {
                channel,
                request: request.clone(),
            }
            .encode(&mut Writer::new(&mut frame));
            if let Err(error) = encoded {
                self.free(channel).await;
                return Err(error);
            }

            if outbound.sender.send(frame).await.is_ok() {
                return Ok((outbound.generation, receiver));
            }

            // The connection died between the wait and the send. Free
            // the tag and wait for the slot to move past that
            // connection before trying again. If the release beat the
            // free, the slot is already empty, and freeing it again
            // is freeing nothing.
            self.free(channel).await;
            self.next(&mut watcher, outbound.generation).await;
        }
    }

    /// Wait until the outbound slot no longer holds `generation` —
    /// a fresh subscription over [`next`](Self::next), for callers
    /// outside the open loop.
    pub async fn wait_past(&self, generation: u64) {
        let mut watcher = self.outbound.subscribe();
        self.next(&mut watcher, generation).await;
    }

    /// Wait until the outbound slot no longer holds `generation`.
    async fn next(
        &self,
        watcher: &mut watch::Receiver<Option<Outbound>>,
        generation: u64,
    ) {
        let _ = watcher
            .wait_for(|slot| match slot {
                Some(current) => current.generation != generation,
                None => true,
            })
            .await;
    }

    /// Route one response to its channel's opener. A response on a
    /// dead tag is dropped — the opener is gone, and there is nobody
    /// to tell.
    pub async fn respond(&self, channel: u8, payload: Bytes) {
        let routes = self.routes.lock().await;
        if let Some(sender) = &routes.slots[channel as usize] {
            let _ = sender.send(ChannelEvent::Response(payload));
        }
    }

    /// End a channel the server's way: tell the opener it finished,
    /// then return the tag to the pool.
    pub async fn finish(&self, channel: u8) {
        let mut routes = self.routes.lock().await;
        if let Some(sender) = routes.slots[channel as usize].take() {
            let _ = sender.send(ChannelEvent::Finished);
        }
        drop(routes);
        self.freed.notify_waiters();
    }

    /// Return a tag to the pool without a finish — the opener's own
    /// cleanup for a request that never made it onto the wire.
    async fn free(&self, channel: u8) {
        let mut routes = self.routes.lock().await;
        routes.slots[channel as usize] = None;
        drop(routes);
        self.freed.notify_waiters();
    }

    /// Allocate the lowest free tag on `generation`'s table, waiting
    /// if all 256 are live. `None` when the table no longer belongs to
    /// that generation — the caller re-waits for a connection rather
    /// than registering a channel nothing would ever end.
    async fn allocate(
        &self,
        generation: u64,
    ) -> Option<(u8, mpsc::UnboundedReceiver<ChannelEvent>)> {
        loop {
            let notified = std::pin::pin!(self.freed.notified());
            {
                let mut routes = self.routes.lock().await;
                if routes.current != generation {
                    return None;
                }
                if let Some(index) =
                    routes.slots.iter().position(Option::is_none)
                {
                    let (sender, receiver) = mpsc::unbounded_channel();
                    routes.slots[index] = Some(sender);
                    return Some((index as u8, receiver));
                }
            }
            notified.await;
        }
    }

    /// The release behind [`Claim`]'s drop, in the order that keeps a
    /// newer connection whole: wipe this generation's routes first —
    /// senders dropped without a finish, which is how their openers
    /// learn the connection died — then clear the outbound slot, then
    /// free the occupancy. A newer connection cannot exist until the
    /// last step, so the wipe can only ever touch its own generation's
    /// table; the guards make the late and the stale into no-ops.
    async fn release(&self, generation: u64) {
        {
            let mut routes = self.routes.lock().await;
            if routes.current == generation {
                routes.current = 0;
                for slot in routes.slots.iter_mut() {
                    *slot = None;
                }
            }
        }
        self.freed.notify_waiters();
        self.outbound.send_if_modified(|slot| match slot {
            Some(outbound) if outbound.generation == generation => {
                *slot = None;
                true
            }
            _ => false,
        });
        {
            let mut occupancy = self.occupancy.lock().await;
            if occupancy.generation == generation {
                occupancy.taken = false;
            }
        }
    }
}

/// The release rides a spawned task, because a drop cannot await and
/// the locks it needs are async. The generation guard is what makes
/// that safe: however late the task runs, it frees only the claim it
/// was spawned for. The one visible consequence is a window in which
/// the slot is still taken after a connection died — an arrival in
/// that window is refused, and the next attempt finds the slot free.
impl Drop for Claim {
    fn drop(&mut self) {
        let proxy = Arc::clone(&self.proxy);
        let generation = self.generation;
        tokio::spawn(async move {
            proxy.release(generation).await;
        });
    }
}
