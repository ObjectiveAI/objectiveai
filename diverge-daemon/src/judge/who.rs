//! Who a connection is served for.

use diverge_sdk::shared::Identity;

use super::Declared;
use crate::store::AccountId;

/// Who is asking, fixed at the handshake: an account, by its id and
/// nothing else — not its identity and not its grants, since both may
/// change while the connection lives, a rename, a role edited, and
/// every request reads them fresh as a [`Standing`](super::Standing);
/// or a tool, standalone or dependency, by what its container declared
/// at its registration, which nothing changes for the run, and which
/// tool it is, whose attached agents attaching and detaching change
/// and every request reads fresh; or an administrator, by its
/// identity, which the configuration fixes for the daemon's life.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Who {
    /// A client's account.
    Account(AccountId),
    /// A tool's container, under the grants it declared.
    Tool(Declared),
    /// An administrator the configuration's `admins` names, by its
    /// identity: no account, and every grant there is.
    Admin(Identity),
}

impl Who {
    /// The account, when this is one.
    pub fn account(&self) -> Option<AccountId> {
        match self {
            Who::Account(id) => Some(*id),
            Who::Tool(_) | Who::Admin(_) => None,
        }
    }
}
