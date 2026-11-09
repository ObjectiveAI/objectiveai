//! The identities connected to this provider that may be listed.

use std::collections::BTreeMap;
use std::net::IpAddr;

use diverge_sdk::provider::endpoints::daemons::list::server::response;
use diverge_sdk::shared::Identity;
use diverge_sdk::shared::description::Description;
use tokio::sync::{Mutex, broadcast};

/// How many nudges a listing may fall behind before it is told it
/// lagged — and lists again, which is all a nudge asks of it anyway.
const NUDGES_BEHIND: usize = 256;

/// Every identity connected to this provider whose authorization frame
/// carried the listable flag `1` or `2`, the description it carried,
/// and where its connection is — the socket's other end, and the dial
/// target when this provider dialled it — what a `daemons::list` on
/// any connection lists, and keeps listing.
///
/// One per provider, shared across every connection, as the
/// [`Acceptors`](super::acceptors::Acceptors) are. The rules:
///
/// 1. **Entered with its connection.** The host enters an identity
///    when a connection under it is past its handshake with the flag
///    `1` or `2` — whichever end dialled — and before the connection is
///    served, with the description of the flag `2`, the socket's other
///    end's IP, and the address this provider dialled it at when it did.
/// 2. **Left with it.** The host lets it go when that connection ends.
///    An identity holds at most one connection to a provider, so one
///    entry is one connection.
/// 3. **Every change nudges.** An entry entered or left is a nudge to
///    every listing [`subscribe`](Self::subscribe)d — carrying
///    nothing, as a [`Changes`](super::changes::Changes) nudge carries
///    little: a listing lists again and tells what differs.
/// 4. **Listed in order.** By issuer, then by subject — the set's own
///    order.
/// 5. **Listening.** An entry is entered not listening; the
///    [`Mailboxes`](super::mailboxes::Mailboxes) mark it listening as a
///    `daemons::listen` opens under its identity, and not as it closes,
///    and each mark that changes it nudges. A mark for an identity not
///    held is nothing.
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
    /// this provider dialled it — and listables with `description`,
    /// until [`leave`](Self::leave).
    pub(crate) async fn enter(&self, identity: &Identity, description: Option<Description>, ip: IpAddr, address: Option<String>) {
        let listed = response::Listed { identity: identity.clone(), description, ip, address, listening: false };
        self.held.lock().await.insert(identity.clone(), listed);
        let _ = self.nudges.send(());
    }

    /// `identity`'s connection is over: nothing lists it any more.
    pub(crate) async fn leave(&self, identity: &Identity) {
        self.held.lock().await.remove(identity);
        let _ = self.nudges.send(());
    }

    /// Mark `identity` listening, or not (rule 5).
    pub(crate) async fn listening(&self, identity: &Identity, listening: bool) {
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

    /// Whether `identity` is listed now: connected with the flag
    /// `1` or `2`.
    pub(crate) async fn is_listed(&self, identity: &Identity) -> bool {
        self.held.lock().await.contains_key(identity)
    }

    /// Every listables identity connected now, in order, each as a
    /// listing's item.
    pub(crate) async fn list(&self) -> BTreeMap<Identity, response::Listed> {
        self.held.lock().await.clone()
    }
}

impl Default for Listables {
    fn default() -> Self {
        Listables::new()
    }
}
