//! Who a connection is served for.

use std::sync::Arc;

use super::Standing;
use crate::store::AccountId;

/// Who is asking, fixed at the handshake: an account, by its id and
/// nothing else — not its identity and not its grants, since both may
/// change while the connection lives, a rename, a role edited, and
/// every request reads them fresh as a [`Standing`]; or a dependency
/// tool, by the standing its template fixed at its deploy, which
/// nothing changes for its life.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Who {
    /// A client's account, or the account a record's container runs
    /// under.
    Account(AccountId),
    /// A dependency tool, under its template's grants.
    Dependency(Arc<Standing>),
}

impl Who {
    /// The account, when this is one.
    pub fn account(&self) -> Option<AccountId> {
        match self {
            Who::Account(id) => Some(*id),
            Who::Dependency(_) => None,
        }
    }
}
