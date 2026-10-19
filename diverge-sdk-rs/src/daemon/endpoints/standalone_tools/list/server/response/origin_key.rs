//! An origin as a key: what never changes of it.

use crate::daemon::reference;

use super::Origin;

/// An [`Origin`] less the members that may change while the tool is
/// listed — a created tool's `mode`, `provider` and `id` — which is
/// what a listing tells one tool from another by. A created tool is
/// its template and its index; a connected one is the daemon, the
/// account, the tool there and the index.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OriginKey {
    /// Made by this daemon: the template's id and the index.
    Created {
        /// The template's id.
        template: String,
        /// The index among the template's tools.
        index: u64,
    },
    /// Somebody else's, joined: the daemon record, the account there,
    /// the tool there and the index.
    Connected {
        /// The daemon record's name.
        daemon: String,
        /// The account's name on that daemon.
        account: String,
        /// The tool as that daemon refers to it.
        tool: reference::Tool,
        /// The index among the connections to that tool.
        index: u64,
    },
}

impl Origin {
    /// This origin as a key.
    pub fn key(&self) -> OriginKey {
        match self {
            Origin::Created { template, index, .. } => OriginKey::Created {
                template: template.clone(),
                index: *index,
            },
            Origin::Connected {
                daemon,
                account,
                tool,
                index,
            } => OriginKey::Connected {
                daemon: daemon.clone(),
                account: account.clone(),
                tool: tool.clone(),
                index: *index,
            },
        }
    }
}
