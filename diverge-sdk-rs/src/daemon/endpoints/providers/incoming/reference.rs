//! Naming a judge afterwards.

use serde::{Deserialize, Serialize};

/// Which judge a request means: a key judge by the identity it names, a
/// hook judge by its resource. Untagged JSON, one object either way,
/// `{"identity":…}` or `{"resource":…}`; an object with members of both
/// does not decode. There is one key judge per identity and one hook
/// judge per resource, so each names exactly one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Reference {
    /// A key judge, by the identity it names.
    Identity {
        /// The identity.
        identity: String,
    },
    /// A hook judge, by its resource.
    Resource {
        /// The resource, by id.
        resource: String,
    },
}
