//! The identities listening for messages on this provider, what waits
//! for each, and every sender's window.

use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use tokio::sync::{Mutex, Notify, oneshot};

use diverge_sdk::provider::endpoints::info::server::response::MessagesCapabilities;
use diverge_sdk::shared::Identity;
use diverge_sdk::shared::description::Description;

use super::listables::Listables;

/// Every identity with a `daemons::listen` open on this provider, the
/// messages queued for each, and the window of every identity that
/// sent. One per provider, shared across every connection's
/// [`handle`](super::handle::handle), as the [`Acceptors`](super::acceptors::Acceptors)
/// are: a sender reaches a receiver on another socket entirely. The
/// rules:
///
/// 1. **One mailbox per identity.** A listen opens the identity's
///    mailbox, and a second while it is open is refused.
/// 2. **Nothing copied.** A queued message is the sender's own request
///    payload, held by reference; a sending connection has at most one
///    send open, so the queues hold at most one message per sending
///    connection.
/// 3. **Closed is drained.** A mailbox closed answers every message
///    still queued [`NotListening`](Outcome::NotListening), and a
///    message posted to it after is refused the same — under the one
///    lock of its queue, so none is left unanswered.
/// 4. **The window.** Each sending identity has at most one window: it
///    begins at the first send it counts and lasts the provider's
///    `seconds`, and holds at most `messages` sends; a send after it
///    begins another. [`admit`](Self::admit) counts.
/// 5. **Stale windows go.** An expired window is replaced at its
///    identity's next send. An identity that disconnects has its window
///    removed at its end — never before, so a sender does not begin
///    again by reconnecting.
/// 6. **Listening is the listables'.** A mailbox opened marks its
///    identity listening in the [`Listables`], and one closed marks it
///    not, each under the mailboxes' lock; and an identity that
///    announces is entered there by [`announce`](Self::announce), under
///    the same lock, listening exactly when its mailbox is open then.
///    So the listing says listening exactly while a mailbox is open,
///    whichever of the listen and the announce opened first, and
///    however often either ends and opens again. An identity that
///    listens and does not announce is in no listing, and its marks
///    are nothing.
/// 7. **Any identity.** A mailbox is opened for, and a message posted
///    by, any connected identity: neither needs its identity announced.
#[derive(Debug)]
pub struct Mailboxes {
    /// The rate: `None` on a provider that relays no messages.
    rate: Option<MessagesCapabilities>,
    /// The listables, told who listens, and who announces (rule 6).
    listables: Arc<Listables>,
    /// The open mailboxes, by their listening identity.
    boxes: Mutex<HashMap<Identity, Arc<Mailbox>>>,
    /// Every sending identity's window (rules 4 and 5).
    windows: Arc<Mutex<HashMap<Identity, Window>>>,
    /// The id of the next message posted.
    next: Mutex<u64>,
}

/// One listening identity's queue.
#[derive(Debug)]
pub struct Mailbox {
    /// The messages waiting, oldest first, and whether the mailbox is
    /// closed (rule 3).
    queue: Mutex<Queue>,
    /// A word that a message was queued.
    arrived: Notify,
}

#[derive(Debug, Default)]
struct Queue {
    waiting: VecDeque<Waiting>,
    closed: bool,
}

/// One message waiting for its receiver.
#[derive(Debug)]
pub struct Waiting {
    /// Its id, by which its sender withdraws it.
    id: u64,
    /// The sending identity.
    pub from: Identity,
    /// The body: a slice of the send's request payload (rule 2).
    pub body: Bytes,
    /// Where its fate goes.
    answer: oneshot::Sender<Outcome>,
}

/// A queued message's fate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The receiver's reader took it.
    Delivered,
    /// The receiver stopped listening before it took it.
    NotListening,
}

/// A message posted: refused, or queued with what waits on its fate.
#[derive(Debug)]
pub enum Posted {
    /// No mailbox is open under the receiver, or it closed.
    NotListening,
    /// Queued: the mailbox, its id there, and its fate to come.
    Queued {
        mailbox: Arc<Mailbox>,
        id: u64,
        fate: oneshot::Receiver<Outcome>,
    },
}

/// One identity's window: when it began, and how many sends it counted.
#[derive(Debug, Clone, Copy)]
struct Window {
    began: Instant,
    count: u32,
}

impl Mailboxes {
    /// Empty, relaying at `rate`, telling `listables` who listens and
    /// who announces.
    pub fn new(rate: Option<MessagesCapabilities>, listables: Arc<Listables>) -> Self {
        Mailboxes {
            rate,
            listables,
            boxes: Mutex::new(HashMap::new()),
            windows: Arc::new(Mutex::new(HashMap::new())),
            next: Mutex::new(0),
        }
    }

    /// Open `identity`'s mailbox (rule 1): `None` when one is open
    /// already, and nothing changed.
    pub async fn open(&self, identity: &Identity) -> Option<Arc<Mailbox>> {
        let mut boxes = self.boxes.lock().await;
        if boxes.contains_key(identity) {
            return None;
        }
        let mailbox = Arc::new(Mailbox { queue: Mutex::new(Queue::default()), arrived: Notify::new() });
        boxes.insert(identity.clone(), Arc::clone(&mailbox));
        // Rule 6, under the lock.
        self.listables.listening(identity, true).await;
        Some(mailbox)
    }

    /// Close `identity`'s mailbox (rule 3): nothing finds it any more,
    /// and every message still queued is answered not listening.
    pub async fn close(&self, identity: &Identity) {
        let mailbox = {
            let mut boxes = self.boxes.lock().await;
            let Some(mailbox) = boxes.remove(identity) else {
                return;
            };
            // Rule 6, under the lock.
            self.listables.listening(identity, false).await;
            mailbox
        };
        let mut queue = mailbox.queue.lock().await;
        queue.closed = true;
        for waiting in queue.waiting.drain(..) {
            let _ = waiting.answer.send(Outcome::NotListening);
        }
    }

    /// Enter `identity` in the listables (rule 6) — connected from
    /// `ip`, dialled at `address` when this provider dialled it, with
    /// `announcement` — listening exactly when its mailbox is open now.
    /// The mailboxes' lock is held from the reading to the entry, so an
    /// [`open`](Self::open) or a [`close`](Self::close) is wholly
    /// before it, and seen, or wholly after, and marks the entry made.
    pub async fn announce(&self, identity: &Identity, announcement: Description, ip: IpAddr, address: Option<String>) {
        let boxes = self.boxes.lock().await;
        self.listables.enter(identity, announcement, ip, address, boxes.contains_key(identity)).await;
    }

    /// Count one send of `from` against its window (rule 4): `Err` with
    /// the seconds until the window ends, rounded up, when it is full,
    /// and nothing counted. A provider with no rate admits nothing, and
    /// is never asked: its sends are `Unsupported` first.
    pub async fn admit(&self, from: &Identity) -> Result<(), u32> {
        let Some(rate) = self.rate else {
            return Err(0);
        };
        let length = Duration::from_secs(u64::from(rate.seconds.get()));
        let now = Instant::now();
        let mut windows = self.windows.lock().await;
        match windows.get_mut(from) {
            Some(window) if now < window.began + length => {
                if window.count < rate.messages.get() {
                    window.count += 1;
                    Ok(())
                } else {
                    let left = (window.began + length) - now;
                    let seconds = left.as_secs() + u64::from(left.subsec_nanos() > 0);
                    Err(u32::try_from(seconds).unwrap_or(u32::MAX))
                }
            }
            // None yet, or expired (rule 5): a window begins at this send.
            _ => {
                windows.insert(from.clone(), Window { began: now, count: 1 });
                Ok(())
            }
        }
    }

    /// Post a message from `from` to `to`'s mailbox (rule 3).
    pub async fn post(&self, from: &Identity, to: &Identity, body: Bytes) -> Posted {
        let Some(mailbox) = self.boxes.lock().await.get(to).cloned() else {
            return Posted::NotListening;
        };
        let id = {
            let mut next = self.next.lock().await;
            let id = *next;
            *next = next.wrapping_add(1);
            id
        };
        let (answer, fate) = oneshot::channel();
        {
            let mut queue = mailbox.queue.lock().await;
            if queue.closed {
                return Posted::NotListening;
            }
            queue.waiting.push_back(Waiting { id, from: from.clone(), body, answer });
        }
        mailbox.arrived.notify_one();
        Posted::Queued { mailbox, id, fate }
    }

    /// `identity`'s connection ended (rule 5): its window removed at
    /// its end — now, when it has ended — by one timer.
    pub async fn left(&self, identity: &Identity) {
        let Some(rate) = self.rate else {
            return;
        };
        let length = Duration::from_secs(u64::from(rate.seconds.get()));
        let Some(began) = self.windows.lock().await.get(identity).map(|window| window.began) else {
            return;
        };
        let windows = Arc::clone(&self.windows);
        let identity = identity.clone();
        tokio::spawn(async move {
            tokio::time::sleep_until(tokio::time::Instant::from_std(began + length)).await;
            let mut windows = windows.lock().await;
            // Only the window that was: one begun since, by a send after
            // a reconnection, runs on.
            if windows.get(&identity).is_some_and(|window| window.began == began) {
                windows.remove(&identity);
            }
        });
    }
}

impl Mailbox {
    /// The oldest message waiting, once there is one. Cancel-safe:
    /// nothing is taken from the queue unless it is returned. A message
    /// whose sender is gone is passed over, unanswered, since nobody
    /// waits on its fate.
    pub async fn next(&self) -> Waiting {
        loop {
            {
                let mut queue = self.queue.lock().await;
                while let Some(waiting) = queue.waiting.pop_front() {
                    if !waiting.answer.is_closed() {
                        return waiting;
                    }
                }
            }
            self.arrived.notified().await;
        }
    }

    /// Withdraw the message `id`: `true` when it was still waiting and
    /// is gone now, `false` when the receiver had it already.
    pub async fn withdraw(&self, id: u64) -> bool {
        let mut queue = self.queue.lock().await;
        match queue.waiting.iter().position(|waiting| waiting.id == id) {
            Some(at) => {
                queue.waiting.remove(at);
                true
            }
            None => false,
        }
    }
}

impl Waiting {
    /// Tell the sender its fate. A sender gone hears nothing.
    pub fn answer(self, outcome: Outcome) {
        let _ = self.answer.send(outcome);
    }
}
