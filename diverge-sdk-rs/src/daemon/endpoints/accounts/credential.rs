//! How a client dials in as an account.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// What judges a client that dials in as the account, as the provider
/// server judges its inbound peers: a key the credential must equal, or
/// a hook that judges it. Untagged JSON, told apart by its members —
/// `{"key":…,"identity":…}` with an `address` if any, or
/// `{"authorize_hook":…}` — and an object with members of both does not
/// decode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Credential {
    /// A credential that must equal a key, byte for byte.
    Key {
        /// The string the credential must equal. Given on a create or
        /// an edit, and answered by nothing.
        key: String,
        /// Who a client that presents the key is: the identity it is
        /// served under from then on, unless the account has a name,
        /// which is then the identity. The key itself never serves as
        /// one. One key account per identity.
        identity: String,
        /// The one peer address the key is accepted from, as the OS
        /// reports it; absent, any address.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        address: Option<IpAddr>,
    },
    /// A hook that judges the credential itself and names the identity.
    Hook {
        /// The directory
        /// [resource](crate::daemon::endpoints::resources) that is the
        /// hook, by id, with `hook.yaml` at its root; held before the
        /// create, which refuses one that is not. See
        /// [`providers`](crate::daemon::endpoints::providers) for what
        /// it is run with and what it answers. One hook account per
        /// resource.
        authorize_hook: String,
    },
}
