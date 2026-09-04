//! The conduit's shared state: one connection slot toward the server,
//! and the table of the agent's connections riding it.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use bytes::Bytes;
use tokio::sync::{Mutex, mpsc, watch};

/// How many frames may queue toward the socket before senders wait.
///
/// A bound, deliberately: what waits on it is a connection's read
/// loop, and a read loop that waits is TCP flow control reaching the
/// agent's driver — the honest signal for a server that cannot keep
/// up.
const OUTBOUND_CAPACITY: usize = 64;

/// The connection slot and the connection table, shared by the
/// WebSocket half and the agent-side connections.
///
/// # One connection toward the server
///
/// [`claim`](Self::claim) is the door: it succeeds for exactly one
/// holder at a time, and the [`Claim`] it returns releases the slot
/// when dropped — including the drop that happens when an upgrade
/// never completes, so a connection that failed to arrive cannot
/// wedge the slot shut.
///
/// # The table belongs to a generation
///
/// The connection table is stamped with the generation of the
/// WebSocket it serves. Registration checks the stamp under the same
/// lock, and release sweeps only its own generation's table — so an
/// agent's connection can neither be registered into a table that was
/// already swept (a socket nothing would ever close) nor swept out of
/// a newer connection's table (a socket closed under a server still
/// carrying it).
///
/// # The locks are short, and never held across the wire
///
/// Every critical section here is a read-modify-write on plain
/// state. What waits — for a WebSocket, for a frame — waits on
/// [`watch`] and the queues, outside any lock.
pub struct Conduit {
    /// Whether a WebSocket currently holds the slot, and the
    /// generation of the latest claim. The generation is what makes
    /// release idempotent: a stale release cannot free a newer
    /// claim's slot.
    occupancy: Mutex<Occupancy>,
    /// The live WebSocket's outbound queue, if any. Connections wait
    /// for `Some`; the WebSocket half publishes on arrival, and
    /// release clears it.
    outbound: watch::Sender<Option<Outbound>>,
    /// The agent's live connections, each the sender that feeds its
    /// socket, stamped with the generation they ride.
    connections: Mutex<Connections>,
    /// The next connection number. Counted up and never reused
    /// within one run of this program, which is the simplest way to
    /// be unique among the live ones.
    next: AtomicU32,
}

/// The sending half of one WebSocket connection.
#[derive(Clone)]
pub struct Outbound {
    /// Which claim this WebSocket is. A connection that saw its send
    /// fail, or the slot move past this, knows the WebSocket it rode
    /// is gone.
    pub generation: u64,
    /// Encoded frames, in order, toward the socket's write pump.
    sender: mpsc::Sender<Vec<u8>>,
}

impl Outbound {
    /// Queue one frame toward the socket. `false` is the WebSocket
    /// gone — the pump dropped its receiver — and nothing else.
    pub async fn send(&self, frame: Vec<u8>) -> bool {
        self.sender.send(frame).await.is_ok()
    }
}

struct Occupancy {
    taken: bool,
    generation: u64,
}

/// The agent's connections and the generation they ride.
///
/// A sender here feeds one agent socket's write pump. Removing it —
/// [`Conduit::close`] for one, the sweep in release for all — drops
/// the sender, the pump sees the end and shuts the socket, and the
/// agent's driver sees a server that hung up.
struct Connections {
    writers: HashMap<u32, mpsc::UnboundedSender<Bytes>>,
    /// The generation whose WebSocket these ride, or `0` for none.
    /// Registration and sweep both check it under the lock.
    current: u64,
}

/// A claimed connection slot, released on drop.
///
/// The WebSocket half holds one for the life of its connection. Its
/// drop is the release, so every exit — a clean close, a read error,
/// an upgrade that never called back — frees the slot and closes the
/// agent's connections that were riding the WebSocket.
pub struct Claim {
    conduit: Arc<Conduit>,
    generation: u64,
}

impl Conduit {
    pub fn new() -> Self {
        Self {
            occupancy: Mutex::new(Occupancy {
                taken: false,
                generation: 0,
            }),
            outbound: watch::Sender::new(None),
            connections: Mutex::new(Connections {
                writers: HashMap::new(),
                current: 0,
            }),
            next: AtomicU32::new(1),
        }
    }

    /// Make the outbound queue a claimed WebSocket will publish.
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
            conduit: Arc::clone(self),
            generation: occupancy.generation,
        })
    }

    /// Publish a claimed WebSocket's outbound queue, unblocking every
    /// waiting connection.
    ///
    /// The table is stamped BEFORE the queue is visible, so a
    /// connection that sees the WebSocket can always register on it.
    /// `send_replace`, not `send`: `send` reports "no receivers" as an
    /// error AND discards the value, and on a cold start there is no
    /// receiver yet — every connection subscribes when it arrives.
    pub async fn publish(&self, claim: &Claim, sender: mpsc::Sender<Vec<u8>>) {
        {
            let mut connections = self.connections.lock().await;
            connections.current = claim.generation;
        }
        self.outbound.send_replace(Some(Outbound {
            generation: claim.generation,
            sender,
        }));
    }

    /// Wait for a WebSocket, and hand back its sending half.
    ///
    /// `wait_for` sees the current value first, so a live WebSocket
    /// is found without an edge — and a connection that arrived
    /// before any WebSocket parks here, its first bytes waiting in
    /// the kernel's buffer, until one does.
    pub async fn attached(&self) -> Outbound {
        let mut watcher = self.outbound.subscribe();
        watcher
            .wait_for(Option::is_some)
            .await
            .expect("the conduit outlives its watchers")
            .clone()
            .expect("checked Some")
    }

    /// Wait until the outbound slot no longer holds `generation`: the
    /// WebSocket a connection rode is gone.
    ///
    /// Resolves at once if it already is, which is the race this
    /// exists for: a connection that registered just as the
    /// WebSocket died would otherwise sit on a loopback socket that
    /// nothing will ever write to.
    pub async fn detached(&self, generation: u64) {
        let mut watcher = self.outbound.subscribe();
        let _ = watcher
            .wait_for(|slot| match slot {
                Some(current) => current.generation != generation,
                None => true,
            })
            .await;
    }

    /// Register one agent connection on `generation`'s table: mint its
    /// number and keep the sender that feeds its socket. `None` when
    /// the table no longer belongs to that generation — the release
    /// already ran — so the caller waits for the next WebSocket rather
    /// than registering a socket nothing would ever close.
    ///
    /// The receiver is the connection's own: what the server sends
    /// for it arrives there, and the stream ending is the connection
    /// being closed from this side.
    pub async fn open(
        &self,
        generation: u64,
    ) -> Option<(u32, mpsc::UnboundedReceiver<Bytes>)> {
        let mut connections = self.connections.lock().await;
        if connections.current != generation {
            return None;
        }
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = mpsc::unbounded_channel();
        connections.writers.insert(id, sender);
        Some((id, receiver))
    }

    /// Route the database's bytes to their connection's socket. Bytes
    /// for a connection that is gone are dropped — the agent's socket
    /// is closed, and there is nobody to tell.
    pub async fn deliver(&self, id: u32, bytes: Bytes) {
        let connections = self.connections.lock().await;
        if let Some(sender) = connections.writers.get(&id) {
            let _ = sender.send(bytes);
        }
    }

    /// Close one agent connection from this side: drop its sender, and
    /// the write pump shuts the socket. Closing one already gone is
    /// closing nothing.
    pub async fn close(&self, id: u32) {
        let mut connections = self.connections.lock().await;
        connections.writers.remove(&id);
    }

    /// The release behind [`Claim`]'s drop, in the order that keeps a
    /// newer WebSocket whole: sweep this generation's table first —
    /// every sender dropped, every agent socket shut — then clear the
    /// outbound slot, then free the occupancy. A newer WebSocket cannot
    /// exist until the last step, so the sweep can only ever touch its
    /// own generation's table; the guards make the late and the stale
    /// into no-ops.
    async fn release(&self, generation: u64) {
        {
            let mut connections = self.connections.lock().await;
            if connections.current == generation {
                connections.current = 0;
                connections.writers.clear();
            }
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
/// was spawned for. The one visible consequence is a window in which
/// the slot is still taken after a WebSocket died — an arrival in that
/// window is refused, and the next attempt finds the slot free.
impl Drop for Claim {
    fn drop(&mut self) {
        let conduit = Arc::clone(&self.conduit);
        let generation = self.generation;
        tokio::spawn(async move {
            conduit.release(generation).await;
        });
    }
}
