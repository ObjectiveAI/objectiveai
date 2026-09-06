//! The `/requests` slot and the table of asks awaiting their answer.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use bytes::Bytes;
use diverge_provider_sdk::container_proxy::requests::request::{
    Frame, Request, RequestEncodeError,
};
use diverge_provider_sdk::encode::{Encode, Writer};
use tokio::sync::{Mutex, mpsc, watch};

/// How many frames may queue toward the `/requests` socket before
/// askers wait.
const OUTBOUND_CAPACITY: usize = 64;

/// How many chunks may queue from a driver toward its `/postgres`
/// path before the driver's read loop waits.
///
/// A bound, deliberately: what waits on it is that read loop, and a
/// read loop that waits is TCP flow control reaching the driver —
/// the honest signal for a caller's database that cannot keep up.
const CONDUIT_CAPACITY: usize = 64;

/// Which answer path an ask is answered on.
///
/// Every [`Request`] kind has one, and an answer path opening for a
/// channel is checked against it: a channel asked as one kind cannot
/// be answered on another kind's path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    McpListTools,
    McpListResources,
    McpCallTool,
    McpReadResource,
    McpNotifications,
    VaultGet,
    VaultSet,
    VaultDelete,
    VaultLock,
    VaultUnlock,
    Command,
    Postgres,
}

impl Kind {
    fn of(request: &Request<'_>) -> Self {
        match request {
            Request::McpListTools(_) => Kind::McpListTools,
            Request::McpListResources(_) => Kind::McpListResources,
            Request::McpCallTool(_) => Kind::McpCallTool,
            Request::McpReadResource(_) => Kind::McpReadResource,
            Request::McpNotifications(_) => Kind::McpNotifications,
            Request::VaultGet(_) => Kind::VaultGet,
            Request::VaultSet(_) => Kind::VaultSet,
            Request::VaultDelete(_) => Kind::VaultDelete,
            Request::VaultLock(_) => Kind::VaultLock,
            Request::VaultUnlock(_) => Kind::VaultUnlock,
            Request::Command(_) => Kind::Command,
            Request::Postgres(_) => Kind::Postgres,
        }
    }
}

/// What reaches an asker from its answer path.
///
/// The three ends an answer can come to, and only two of them are
/// the server's: a clean close after zero or more messages is
/// [`Complete`](Event::Complete); anything else — the answer socket
/// dropping, the `/requests` socket dying before the answer path
/// ever opened — is [`Died`](Event::Died), which each kind of ask
/// treats by its own rule.
///
/// One path is not an answer but a conversation: `/postgres` carries
/// bytes both ways. Its asker hears [`Opened`](Event::Opened) first,
/// with the sender that writes on the path, before any message; no
/// other path sends it, and the other askers ignore it.
#[derive(Debug, Clone)]
pub enum Event {
    /// The path opened, and this is how to write on it. The postgres
    /// path's alone.
    Opened(mpsc::Sender<Bytes>),
    /// One message of the answer.
    Message(Bytes),
    /// The answer path closed cleanly: the answer is whole.
    Complete,
    /// The answer will never be whole.
    Died,
}

/// The connection slot for `/requests`, and every ask in flight.
///
/// # One connection
///
/// [`claim`](Self::claim) is the door: it succeeds for exactly one
/// holder at a time, and the [`Claim`] it returns releases the slot
/// when dropped — including the drop that happens when an upgrade
/// never completes, so a connection that failed to arrive cannot
/// wedge the slot shut.
///
/// # An ask belongs to the connection it was sent on — until answered
///
/// The table stamps each ask with the generation of the `/requests`
/// connection that carried it. When that connection dies, the asks
/// whose answer path has NOT opened are dead too: the server may
/// never have read them. The asks whose answer path HAS opened live
/// on, because the answer socket is its own connection and the
/// server is already speaking on it.
///
/// # The locks are short, and never held across the wire
///
/// Every critical section is a read-modify-write on plain state.
/// What waits — for a connection, for a message — waits on
/// [`watch`] and the queues, outside any lock.
pub struct Requests {
    /// Whether a connection holds the slot, and the generation of
    /// the latest claim; the generation makes release idempotent.
    occupancy: Mutex<Occupancy>,
    /// The live connection's outbound queue, if any. Askers wait for
    /// `Some`; the WebSocket half publishes on arrival, and release
    /// clears it.
    outbound: watch::Sender<Option<Outbound>>,
    /// The asks in flight, by channel.
    asks: Mutex<Asks>,
    /// The next channel. Counted up and never reused within one run
    /// of this program, which is the simplest way to be unique among
    /// the asks not yet answered.
    next: AtomicU32,
}

/// The sending half of one `/requests` connection.
#[derive(Clone)]
struct Outbound {
    generation: u64,
    sender: mpsc::Sender<Vec<u8>>,
}

struct Occupancy {
    taken: bool,
    generation: u64,
}

/// The asks in flight and the generation whose connection carried
/// the latest of them.
struct Asks {
    slots: HashMap<u32, Slot>,
    /// The generation of the live connection, or `0` for none.
    current: u64,
}

/// One ask awaiting its answer.
struct Slot {
    /// Which path may answer it.
    kind: Kind,
    /// The connection it was sent on.
    generation: u64,
    /// Whether its answer path has opened.
    opened: bool,
    /// Where its events go: the asker holds the other end.
    events: mpsc::UnboundedSender<Event>,
}

/// A claimed `/requests` slot, released on drop.
pub struct Claim {
    requests: Arc<Requests>,
    generation: u64,
}

/// An answer path serving one ask: what the WebSocket half holds
/// while it reads the answer.
pub struct Answering {
    channel: u32,
    events: mpsc::UnboundedSender<Event>,
}

/// Why an answer path was refused before its upgrade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// No ask with that channel, or not one this path answers.
    NotFound,
    /// An answer path for that channel is already open.
    Conflict,
}

impl Requests {
    pub fn new() -> Self {
        Self {
            occupancy: Mutex::new(Occupancy {
                taken: false,
                generation: 0,
            }),
            outbound: watch::Sender::new(None),
            asks: Mutex::new(Asks {
                slots: HashMap::new(),
                current: 0,
            }),
            next: AtomicU32::new(1),
        }
    }

    /// Make the outbound queue a claimed connection will publish.
    pub fn queue() -> (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) {
        mpsc::channel(OUTBOUND_CAPACITY)
    }

    /// Make the queue a duplex path hands its asker: the driver's
    /// chunks, toward the path's socket.
    pub fn conduit() -> (mpsc::Sender<Bytes>, mpsc::Receiver<Bytes>) {
        mpsc::channel(CONDUIT_CAPACITY)
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
            requests: Arc::clone(self),
            generation: occupancy.generation,
        })
    }

    /// Publish a claimed connection's outbound queue, unblocking every
    /// waiting asker.
    ///
    /// The table is stamped BEFORE the queue is visible, so an asker
    /// that sees the connection can always register on it.
    /// `send_replace`, not `send`: `send` reports "no receivers" as an
    /// error AND discards the value, and on a cold start there is no
    /// receiver yet — every asker subscribes when it arrives.
    pub async fn publish(&self, claim: &Claim, sender: mpsc::Sender<Vec<u8>>) {
        {
            let mut asks = self.asks.lock().await;
            asks.current = claim.generation;
        }
        self.outbound.send_replace(Some(Outbound {
            generation: claim.generation,
            sender,
        }));
    }

    /// Send one ask on the live connection — waiting for one if none
    /// is live, and moving to the next if the one it tried died under
    /// it — and hand back where its answer's events will arrive,
    /// with the generation of the connection it went on.
    ///
    /// `Err` is the one failure that is this side's own: an ask whose
    /// payload would not encode, which retrying cannot change.
    pub async fn ask(
        &self,
        request: Request<'_>,
    ) -> Result<(u64, mpsc::UnboundedReceiver<Event>), RequestEncodeError> {
        let mut watcher = self.outbound.subscribe();
        loop {
            // `wait_for` sees the current value first, so a live
            // connection is found without an edge.
            let outbound = watcher
                .wait_for(Option::is_some)
                .await
                .expect("the table outlives its watchers")
                .clone()
                .expect("checked Some");

            // Register on the connection's table — or, if the release
            // already ran and the table moved on, wait for the next
            // connection like any other asker.
            let (channel, receiver) = {
                let mut asks = self.asks.lock().await;
                if asks.current != outbound.generation {
                    drop(asks);
                    self.next(&mut watcher, outbound.generation).await;
                    continue;
                }
                let channel = self.next.fetch_add(1, Ordering::Relaxed);
                let (sender, receiver) = mpsc::unbounded_channel();
                asks.slots.insert(
                    channel,
                    Slot {
                        kind: Kind::of(&request),
                        generation: outbound.generation,
                        opened: false,
                        events: sender,
                    },
                );
                (channel, receiver)
            };

            let mut frame = Vec::new();
            let encoded = Frame {
                channel,
                request: request.clone(),
            }
            .encode(&mut Writer::new(&mut frame));
            if let Err(error) = encoded {
                self.forget(channel).await;
                return Err(error);
            }

            if outbound.sender.send(frame).await.is_ok() {
                return Ok((outbound.generation, receiver));
            }

            // The connection died between the wait and the send.
            // Forget the ask and wait for the slot to move past that
            // connection before trying again.
            self.forget(channel).await;
            self.next(&mut watcher, outbound.generation).await;
        }
    }

    /// Wait until the outbound slot no longer holds `generation` —
    /// for a caller that wants to outwait the connection its ask
    /// went on.
    pub async fn wait_past(&self, generation: u64) {
        let mut watcher = self.outbound.subscribe();
        self.next(&mut watcher, generation).await;
    }

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

    /// An answer path opening for `channel`: admit it if the channel
    /// is an ask of this `kind` not yet being answered.
    pub async fn open(
        &self,
        kind: Kind,
        channel: u32,
    ) -> Result<Answering, Refusal> {
        let mut asks = self.asks.lock().await;
        let Some(slot) = asks.slots.get_mut(&channel) else {
            return Err(Refusal::NotFound);
        };
        if slot.kind != kind {
            return Err(Refusal::NotFound);
        }
        if slot.opened {
            return Err(Refusal::Conflict);
        }
        slot.opened = true;
        Ok(Answering {
            channel,
            events: slot.events.clone(),
        })
    }

    /// One message of an answer, to its asker. An asker that has
    /// gone — dropped its receiver — is nobody to tell.
    pub fn deliver(&self, answering: &Answering, bytes: Bytes) {
        let _ = answering.events.send(Event::Message(bytes));
    }

    /// A duplex path opened: hand its asker the sender that writes on
    /// it, before any message. An asker that has gone is nobody to
    /// tell, and the sender drops with the event — the path then sees
    /// its queue end, which is the driver gone.
    pub fn deliver_opened(&self, answering: &Answering, sender: mpsc::Sender<Bytes>) {
        let _ = answering.events.send(Event::Opened(sender));
    }

    /// The answer path ended: whole if it closed cleanly, dead
    /// otherwise. The ask is over either way.
    pub async fn finish(&self, answering: Answering, complete: bool) {
        {
            let mut asks = self.asks.lock().await;
            asks.slots.remove(&answering.channel);
        }
        let _ = answering.events.send(if complete {
            Event::Complete
        } else {
            Event::Died
        });
    }

    /// Drop an ask that never went out.
    async fn forget(&self, channel: u32) {
        let mut asks = self.asks.lock().await;
        asks.slots.remove(&channel);
    }

    /// The release behind [`Claim`]'s drop, in the order that keeps a
    /// newer connection whole: kill this generation's asks that no
    /// answer path has opened — their senders dropped after a
    /// [`Died`](Event::Died) — then clear the outbound slot, then
    /// free the occupancy. A newer connection cannot exist until the
    /// last step; an ask already being answered is left alone, since
    /// its answer socket is its own.
    async fn release(&self, generation: u64) {
        {
            let mut asks = self.asks.lock().await;
            if asks.current == generation {
                asks.current = 0;
            }
            asks.slots.retain(|_, slot| {
                if slot.generation == generation && !slot.opened {
                    let _ = slot.events.send(Event::Died);
                    false
                } else {
                    true
                }
            });
        }
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
/// was spawned for.
impl Drop for Claim {
    fn drop(&mut self) {
        let requests = Arc::clone(&self.requests);
        let generation = self.generation;
        tokio::spawn(async move {
            requests.release(generation).await;
        });
    }
}
