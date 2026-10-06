//! One way of judging a provider that dials the daemon.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// One credential of an incoming provider, as the provider server has
/// them: a key the presented credential must equal, or a hook that
/// judges it.
/// Untagged JSON, told apart by its members as the server's are —
/// `{"key":…,"identity":…}` with an `address` if any, or
/// `{"authorize_hook":…}` — and an object with members of both does not
/// decode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Credential {
    /// A credential that must equal a key, byte for byte.
    Key {
        /// The string the credential must equal. Given on an add or an
        /// edit, and answered by nothing.
        key: String,
        /// Who a peer that presents the key is: the provider's identity
        /// from then on. The key itself never serves as one. One key
        /// credential per identity.
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
        /// hook, by id, with `hook.yaml` at its root; held by the
        /// caller before the add, which refuses one that is not. See
        /// [`providers`](crate::daemon::endpoints::providers) for what
        /// it is run with and what it answers. One hook credential per
        /// resource.
        authorize_hook: String,
    },
}
