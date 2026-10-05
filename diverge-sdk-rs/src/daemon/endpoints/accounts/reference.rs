//! Naming an account afterwards.

use serde::{Deserialize, Serialize};

/// Which account a request means: by its name, by the identity its key
/// credential names, or by the resource its hook credential is.
/// Untagged JSON, one object any way, `{"name":…}`, `{"identity":…}` or
/// `{"authorize_hook":…}`; an object with members of more than one does
/// not decode. There is one account per name, one key account per
/// identity and one hook account per resource, so each names exactly
/// one, and a named account with a key credential is named by either.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Reference {
    /// By name.
    Name {
        /// The name.
        name: String,
    },
    /// A key account, by the identity its key names.
    Identity {
        /// The identity.
        identity: String,
    },
    /// A hook account, by its resource.
    AuthorizeHook {
        /// The resource, by id.
        authorize_hook: String,
    },
}
