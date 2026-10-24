//! Who a connection is served for.

use diverge_sdk::shared::Identity;

use super::Dependent;
use crate::store::AccountId;

/// Who is asking, fixed at the handshake: an account, by its id and
/// nothing else — not its identity and not its grants, since both may
/// change while the connection lives, a rename, a role edited, and
/// every request reads them fresh as a [`Standing`](super::Standing);
/// or a dependency tool, by what its template fixed at its deploy,
/// which nothing changes for its life, and whose `self` it is, which
/// a service's attachments change and every request reads fresh; or an
/// administrator, by its identity, which the configuration fixes for
/// the daemon's life.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Who {
    /// A client's account, or the account a tool record's container
    /// runs under.
    Account(AccountId),
    /// A dependency tool, under its template's grants.
    Dependency(Dependent),
    /// An administrator the configuration's `admins` names, by its
    /// identity: no account, and every grant there is.
    Admin(Identity),
}

impl Who {
    /// The account, when this is one.
    pub fn account(&self) -> Option<AccountId> {
        match self {
            Who::Account(id) => Some(*id),
            Who::Dependency(_) | Who::Admin(_) => None,
        }
    }
}
