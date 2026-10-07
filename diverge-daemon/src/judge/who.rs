//! The account a connection is served for.

use crate::store::AccountId;

/// Who is asking, fixed at the handshake: the account's id and
/// nothing else. Not its identity and not its grants, since both may
/// change while the connection lives — a rename, a role edited — and
/// every request reads them fresh as a [`Standing`](super::Standing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Who {
    /// The account.
    pub id: AccountId,
}
