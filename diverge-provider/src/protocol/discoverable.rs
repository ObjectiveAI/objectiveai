//! The identities connected to this provider that may be listed.

use std::collections::BTreeMap;
use std::net::IpAddr;

use diverge_sdk::provider::endpoints::discover::server::response;
use diverge_sdk::shared::Identity;
use diverge_sdk::shared::discover::Description;
use tokio::sync::{Mutex, broadcast};

/// How many nudges a listing may fall behind before it is told it
/// lagged — and lists again, which is all a nudge asks of it anyway.
const NUDGES_BEHIND: usize = 256;

/// Every identity connected to this provider whose authorization frame
/// carried the discover flag `1` or `2`, the description it carried,
/// and where its connection is — the socket's other end, and the dial
/// target when this provider dialled it — what a `discover` listing on
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
#[derive(Debug)]
pub struct Discoverable {
    held: Mutex<BTreeMap<Identity, response::Discoverable>>,
    nudges: broadcast::Sender<()>,
}

impl Discoverable {
    pub fn new() -> Self {
        let (nudges, _) = broadcast::channel(NUDGES_BEHIND);
        Discoverable { held: Mutex::new(BTreeMap::new()), nudges }
    }

    /// A nudge for every change from now on — subscribed before a
    /// listing reads, so that nothing between the read and the
    /// subscription is missed.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.nudges.subscribe()
    }

    /// `identity` is connected from `ip` — dialled at `address`, when
    /// this provider dialled it — and discoverable with `description`,
    /// until [`leave`](Self::leave).
    pub(crate) async fn enter(&self, identity: &Identity, description: Option<Description>, ip: IpAddr, address: Option<String>) {
        let listed = response::Discoverable { identity: identity.clone(), description, ip, address };
        self.held.lock().await.insert(identity.clone(), listed);
        let _ = self.nudges.send(());
    }

    /// `identity`'s connection is over: nothing lists it any more.
    pub(crate) async fn leave(&self, identity: &Identity) {
        self.held.lock().await.remove(identity);
        let _ = self.nudges.send(());
    }

    /// Every discoverable identity connected now, in order, each as a
    /// listing's item.
    pub(crate) async fn list(&self) -> BTreeMap<Identity, response::Discoverable> {
        self.held.lock().await.clone()
    }
}

impl Default for Discoverable {
    fn default() -> Self {
        Discoverable::new()
    }
}
