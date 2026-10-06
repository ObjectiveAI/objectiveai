//! Naming an account afterwards.

use serde::{Deserialize, Serialize};

/// Which account a request means: by its name, or by the identity its
/// credential names. Untagged JSON, one object either way, `{"name":…}`
/// or `{"identity":…}`; an object with both does not decode. There is
/// one account per name and one credential per identity, so each names
/// exactly one, and a named account with a credential is named by
/// either.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Reference {
    /// By name.
    Name {
        /// The name.
        name: String,
    },
    /// By the identity its credential names.
    Identity {
        /// The identity.
        identity: String,
    },
}
