//! Naming a credential afterwards.

use serde::{Deserialize, Serialize};

/// Which credential a request means: a key credential by the identity
/// it names, a hook credential by its resource. Untagged JSON, one
/// object either way, `{"identity":…}` or `{"authorize_hook":…}`, the
/// members a [`Credential`](super::Credential) is told apart by; an
/// object with both does not decode. There is one key credential per
/// identity and one hook credential per resource, so each names exactly
/// one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Reference {
    /// A key credential, by the identity it names.
    Identity {
        /// The identity.
        identity: String,
    },
    /// A hook credential, by its resource.
    AuthorizeHook {
        /// The resource, by id.
        authorize_hook: String,
    },
}
