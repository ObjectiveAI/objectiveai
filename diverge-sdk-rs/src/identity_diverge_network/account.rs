//! Which account.

use std::fmt;

/// An account: one user of one external provider. Written
/// `<provider>.<id>`, it is the first two parts of every subject at
/// `identity.diverge.network` that is the account's own.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Account {
    /// The provider's name.
    pub provider: String,
    /// The user's id at the provider: the one it never changes, as
    /// text.
    pub id: String,
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.provider, self.id)
    }
}

/// Whether `id` may be an account's: from one to 255 ASCII letters,
/// digits, `-` and `_`. Every provider's is — GitHub's and Google's
/// are digits — and a file is named by it as it is, so one that is
/// not is refused, and names nothing.
pub(super) fn usable(id: &str) -> bool {
    (1..=255).contains(&id.len()) && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}
