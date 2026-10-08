//! The word that a kind's records or live state changed, for the
//! lists kept open.

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use tokio::sync::broadcast;

/// How many words a listing may fall behind by before it is told so,
/// and reads again instead of the words it missed.
const CHANGES_BEHIND: usize = 256;

/// One kind of thing a list is over: what a word names, and what a
/// listing of that kind listens for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Accounts: the records, and which have a client connected.
    Accounts,
    /// Roles: the records.
    Roles,
    /// Outgoing providers: the records, which are connected, and when
    /// one last was.
    ProvidersOutgoing,
    /// Incoming credentials: the records, and which have a provider
    /// connected through them.
    ProvidersIncoming,
    /// Agent templates: the records, and which are in use.
    AgentsTemplates,
    /// Tool templates: the records, and which are in use.
    ToolsTemplates,
    /// Resources: the records, and which are in use.
    Resources,
    /// Agents: the records, what is attached, which run.
    Agents,
    /// Tools: the records, what is attached, admitted and routed,
    /// which run.
    Tools,
    /// Routes: the records.
    Routes,
    /// Volumes: a provider's listing, and the records that mount one.
    Volumes,
    /// The container connections open through the database.
    Postgres,
}

impl Kind {
    /// The kind a provider's connection is listed under, by its
    /// identity.
    pub fn of_provider(identity: &Identity) -> Kind {
        match identity {
            Identity::Outgoing { .. } => Kind::ProvidersOutgoing,
            Identity::IncomingUnbrokered { .. } => Kind::ProvidersIncoming,
        }
    }
}

/// Every change to any kind, as the handlers and the live state make
/// them, for every list scope kept open.
///
/// A word carries the kind and nothing else: a listing that hears its
/// kind reads the records and the live state again and tells the
/// difference by key, so a word for a change the listing has read
/// already is nothing, and a listing that fell behind reads again
/// the same way. Every handler says the word after its commit; the
/// live state says it at every transition that reaches an item —
/// which is why a listing subscribes before it reads, so a change
/// between the reading and the hearing is in one or the other.
#[derive(Debug)]
pub struct Changes {
    sender: broadcast::Sender<Kind>,
}

impl Changes {
    /// The kind changed. With no listing open, nothing.
    pub fn changed(&self, kind: Kind) {
        let _ = self.sender.send(kind);
    }

    /// Every word from now on, for a listing: subscribed before the
    /// records are read.
    pub fn subscribe(&self) -> broadcast::Receiver<Kind> {
        self.sender.subscribe()
    }
}

impl Default for Changes {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(CHANGES_BEHIND);
        Changes { sender }
    }
}
