//! The providers' message listens, held for each messageable
//! provider's connection, and merged into one ring.

use std::collections::VecDeque;
use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::provider::endpoints::daemons::listen::client::{execute, request};
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::client::handle::Handle;
use futures_util::StreamExt as _;
use tokio::sync::{Mutex, watch};
use tokio::task::JoinHandle;

use crate::daemon::{CHANGES_BEHIND, Daemon};

/// How many messages the ring keeps: as many as a listing may fall
/// behind its words by, [`CHANGES_BEHIND`], the one depth every shared
/// source of the daemon's keeps.
const KEPT: usize = CHANGES_BEHIND;

/// The ring every messageable provider's listen feeds. One per daemon,
/// in its live state. The rules:
///
/// 1. **Held by the record, not by the listeners.** A provider's
///    listen is opened ([`listen`]) for a connected provider whose
///    record says `messageable` and whose capabilities hold out
///    `messages`, and held for the connection's life or until the
///    record says otherwise — see [`sync`](crate::providers::sync),
///    which alone opens and ends it. No client listening opens one,
///    and none leaving ends one.
/// 2. **One per provider.** A provider refuses a second listen on one
///    connection while one is open, so the next is opened only once
///    the one before it there has finished: a listen let go
///    ([`Listen::end`]) is cancelled and read to its finish first, and
///    the sync that ends it opens nothing in its place until then.
/// 3. **Taken at once.** A listen takes each message the moment it
///    arrives, into the ring, whether or not any listener reads: no
///    listener ever holds a provider back, and the provider tells the
///    sender delivered as the daemon takes it — with a client of the
///    daemon's listening then, or with none.
/// 4. **The ring.** Every message taken is numbered, from `1`, and
///    kept; past [`KEPT`] the oldest goes. [`latest`](Self::latest) is
///    the newest number, `0` before any.
/// 5. **Each listener at its own pace.** A listener reads the messages
///    after its own number ([`after`](Self::after)), as many as it can
///    send; one whose number fell out of the ring is told how many it
///    missed, and goes on from the oldest kept.
/// 6. **Ends.** A listen refused — the provider relays no messages, or
///    one is open under the daemon's identity already — failed, or
///    finished ends, and is not opened again while its record's word
///    stands: nothing is retried.
#[derive(Debug)]
pub struct Listening {
    ring: Mutex<Ring>,
    latest: watch::Sender<u64>,
}

impl Default for Listening {
    fn default() -> Self {
        Listening {
            ring: Mutex::new(Ring::default()),
            latest: watch::channel(0).0,
        }
    }
}

/// The messages kept (rule 4).
#[derive(Debug, Default)]
struct Ring {
    /// The newest number given.
    newest: u64,
    /// The messages kept, oldest first, each with its number.
    kept: VecDeque<(u64, Arc<Kept>)>,
}

/// One message taken: the provider it came through, who sent it, and
/// its body, shared and never copied.
#[derive(Debug)]
pub struct Kept {
    /// The provider it came through.
    pub provider: Identity,
    /// The identity of the sending connection there.
    pub from: Identity,
    /// The body.
    pub body: Bytes,
}

/// One provider's listen, held open: what [`listen`] hands back, for
/// [`sync`](crate::providers::sync) to keep with the connection.
/// Dropped, nothing is sent: the listen goes on to the connection's
/// end, which is the one drop there is.
#[derive(Debug)]
pub struct Listen {
    /// Its cancel.
    cancel: execute::Cancel,
    /// The task taking its messages: done once the provider has
    /// finished the scope, or the connection has gone.
    taking: JoinHandle<()>,
}

impl Listen {
    /// Let the listen go (rule 2): cancelled, and waited on until it
    /// has been read to its finish, every message still arriving taken
    /// all the same.
    pub async fn end(self) {
        self.cancel.cancel().await;
        let _ = self.taking.await;
    }
}

impl Listening {
    /// The newest number, to wait on (rule 4).
    pub fn latest(&self) -> watch::Receiver<u64> {
        self.latest.subscribe()
    }

    /// The messages after `cursor` (rule 5): how many it missed, out of
    /// the ring, and every one kept after it, in order.
    pub async fn after(&self, cursor: u64) -> (u64, Vec<(u64, Arc<Kept>)>) {
        let ring = self.ring.lock().await;
        let skipped = match ring.kept.front() {
            Some((oldest, _)) if *oldest > cursor + 1 => *oldest - cursor - 1,
            _ => 0,
        };
        let messages = ring.kept.iter().filter(|(number, _)| *number > cursor).map(|(number, kept)| (*number, Arc::clone(kept))).collect();
        (skipped, messages)
    }

    /// One message taken (rules 3 and 4): numbered, kept, the oldest
    /// let go past [`KEPT`], and the newest number told.
    async fn take(&self, kept: Kept) {
        let newest = {
            let mut ring = self.ring.lock().await;
            ring.newest += 1;
            let number = ring.newest;
            ring.kept.push_back((number, Arc::new(kept)));
            while ring.kept.len() > KEPT {
                ring.kept.pop_front();
            }
            number
        };
        self.latest.send_replace(newest);
    }
}

/// Open `provider`'s listen on `handle` (rule 1), its task spawned:
/// every message taken into the daemon's ring as it arrives (rule 3),
/// until the listen is refused, fails or finishes (rule 6). `None`
/// when the request does not go out.
pub async fn listen(daemon: &Arc<Daemon>, provider: Identity, handle: &Handle) -> Option<Listen> {
    let (mut listen, cancel) = execute::execute(handle, &request::Frame).await.ok()?;
    let daemon = Arc::clone(daemon);
    let taking = tokio::spawn(async move {
        // Rule 6: refused, failed, or finished is the end.
        while let Some(Ok(message)) = listen.next().await {
            daemon.live.listening.take(Kept { provider: provider.clone(), from: message.from, body: message.body }).await;
        }
    });
    Some(Listen { cancel, taking })
}
