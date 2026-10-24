//! The word that one of a caller's named things changed, for the
//! listings open: how a volume listing hears of a create, an edit or
//! a delete, and an image listing of a build or a delete.

use diverge_sdk::shared::Identity;
use std::sync::Arc;

use tokio::sync::broadcast;

/// How many changes a listing may fall behind by before it is told
/// so, and reads the manager again instead of the changes it missed.
const CHANGES_BEHIND: usize = 256;

/// One change: whose, and which name.
#[derive(Debug, Clone)]
pub struct Changed {
    /// Whose thing.
    pub identity: Arc<Identity>,
    /// Which thing, by name.
    pub name: String,
}

/// Every change to any caller's names of one kind, told to every
/// listing of that kind open. One per kind, per provider — one for
/// volumes, one for images — shared by the handlers that make and
/// unmake a name and the listings that watch: a handler that
/// created, edited, built or deleted says [`changed`](Self::changed),
/// and a listing that [`subscribe`](Self::subscribe)d hears a name
/// and an identity, reads its manager again, and tells the
/// difference.
///
/// # A nudge, not a delta
///
/// What travels is that something changed, not what: the listing
/// reads the manager and diffs, so a change the manager made on its
/// own account is told the same way as one a request made, and a
/// listing that fell `CHANGES_BEHIND` changes behind reads the
/// manager again instead of replaying what it missed. The list
/// handler subscribes before it reads the manager, so a change
/// between the reading and the hearing is either in what was read or
/// in what is heard.
#[derive(Debug)]
pub struct Changes {
    /// The changes, to every listing open.
    changes: broadcast::Sender<Changed>,
}

impl Changes {
    /// Nothing changed yet, nobody listening.
    pub fn new() -> Self {
        let (changes, _) = broadcast::channel(CHANGES_BEHIND);
        Changes { changes }
    }

    /// Every change from now on, for a listing: subscribed before the
    /// manager is read.
    pub fn subscribe(&self) -> broadcast::Receiver<Changed> {
        self.changes.subscribe()
    }

    /// The identity's thing by that name was made, changed or
    /// unmade. With no listing open, nothing.
    pub fn changed(&self, identity: &Identity, name: &str) {
        let _ = self.changes.send(Changed {
            identity: Arc::new(identity.clone()),
            name: name.to_string(),
        });
    }
}

impl Default for Changes {
    fn default() -> Self {
        Changes::new()
    }
}
