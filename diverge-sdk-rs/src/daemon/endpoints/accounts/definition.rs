//! What an account is: a name, a credential, or both.

use serde::{Deserialize, Serialize};

use super::Credential;

/// The two shapes an account takes, and between them the rule that an
/// account has a name, a credential, or both, and never neither.
/// Untagged JSON, told apart by whether `name` is there: `{"name":…}`
/// with a `credential` if any, or `{"credential":…}` alone; an object
/// with neither does not decode. Flattened into a
/// [create](crate::daemon::endpoints::accounts::create), so its members
/// are the request's own.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Definition {
    /// An account with a name: what a container's
    /// [`account`](crate::daemon::create::Inner::account) names, and
    /// what a request names the account by. With a credential, a client
    /// dials in as it too.
    Named {
        /// The name, unique among accounts; compared and not read.
        name: String,
        /// How a client dials in as the account, if any client may: see
        /// [`Credential`]. Absent, none does.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        credential: Option<Credential>,
    },
    /// An account without a name: a client dials in as it, and no
    /// container runs under it. Named afterwards by its credential's
    /// identity.
    Unnamed {
        /// How a client dials in as the account: see [`Credential`].
        credential: Credential,
    },
}
