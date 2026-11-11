//! The identities connected to this provider that are announced.

use std::collections::BTreeMap;
use std::net::IpAddr;

use diverge_sdk::provider::endpoints::daemons::list::server::response;
use diverge_sdk::shared::Identity;
use diverge_sdk::shared::description::Description;
use tokio::sync::{Mutex, broadcast};

/// How many nudges a listing may fall behind before it is told it
/// lagged — and lists again, which is all a nudge asks of it anyway.
const NUDGES_BEHIND: usize = 256;

/// Every identity connected to this provider that holds a
/// `daemons::announce` open, the announcement that scope carries, and
/// where its connection is — the socket's other end, and the dial
/// target when this provider dialled it — what a `daemons::list` on
/// any connection lists, and keeps listing.
///
/// One per provider, shared across every connection, as the
/// [`Acceptors`](super::acceptors::Acceptors) are. The rules:
///
/// 1. **Entered with its announce.** An identity is entered when a
///    `daemons::announce` opens on its connection — whichever end
///    dialled — with the request's announcement, the socket's other
///    end's IP, and the address this provider dialled it at when it did.
/// 2. **Left with it.** It is let go when that scope ends: at the
///    caller's cancel, or at its connection's end. An identity holds at
///    most one connection to a provider, and a connection at most one
///    announce, so one entry is one announce scope. An identity that
///    announces again is entered again, as another item.
/// 3. **Every change nudges.** An entry entered or left is a nudge to
///    every listing [`subscribe`](Self::subscribe)d — carrying
///    nothing, as a [`Changes`](super::changes::Changes) nudge carries
///    little: a listing lists again and tells what differs.
/// 4. **Listed in order.** By issuer, then by subject — the set's own
///    order.
/// 5. **Listening.** An entry is entered listening exactly when a
///    `daemons::listen` is open under its identity then; the
///    [`Mailboxes`](super::mailboxes::Mailboxes) mark it listening as
///    one opens after, and not as it closes, and each mark that changes
///    it nudges. A mark for an identity not held — one that listens and
///    does not announce — is nothing.
/// 6. **Entered by the mailboxes.** [`enter`](Self::enter) is called by
///    [`Mailboxes::announce`](super::mailboxes::Mailboxes::announce)
///    alone, and [`listening`](Self::listening) by the mailboxes alone,
///    each under the mailboxes' lock, taken before this one's: whether
///    the identity listens is read and the entry made as one step, and
///    no mark falls between the two.
#[derive(Debug)]
pub struct Listables {
    held: Mutex<BTreeMap<Identity, response::Listed>>,
    nudges: broadcast::Sender<()>,
}

impl Listables {
    pub fn new() -> Self {
        let (nudges, _) = broadcast::channel(NUDGES_BEHIND);
        Listables { held: Mutex::new(BTreeMap::new()), nudges }
    }

    /// A nudge for every change from now on — subscribed before a
    /// listing reads, so that nothing between the read and the
    /// subscription is missed.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.nudges.subscribe()
    }

    /// `identity` is connected from `ip` — dialled at `address`, when
    /// this provider dialled it — and listed with `announcement`,
    /// `listening` or not as it is now (rule 5), until
    /// [`leave`](Self::leave). Called under the mailboxes' lock
    /// (rule 6).
    pub(super) async fn enter(&self, identity: &Identity, announcement: Description, ip: IpAddr, address: Option<String>, listening: bool) {
        let listed = response::Listed { identity: identity.clone(), announcement, ip, address, listening };
        self.held.lock().await.insert(identity.clone(), listed);
        let _ = self.nudges.send(());
    }

    /// `identity`'s announce is over: nothing lists it any more. An
    /// identity not held is nothing, and nudges nobody.
    pub(crate) async fn leave(&self, identity: &Identity) {
        if self.held.lock().await.remove(identity).is_some() {
            let _ = self.nudges.send(());
        }
    }

    /// Mark `identity` listening, or not (rule 5). Called under the
    /// mailboxes' lock (rule 6).
    pub(super) async fn listening(&self, identity: &Identity, listening: bool) {
        let changed = match self.held.lock().await.get_mut(identity) {
            Some(listed) if listed.listening != listening => {
                listed.listening = listening;
                true
            }
            _ => false,
        };
        if changed {
            let _ = self.nudges.send(());
        }
    }

    /// Every identity announced now, in order, each as a listing's
    /// item.
    pub(crate) async fn list(&self) -> BTreeMap<Identity, response::Listed> {
        self.held.lock().await.clone()
    }
}

impl Default for Listables {
    fn default() -> Self {
        Listables::new()
    }
}
