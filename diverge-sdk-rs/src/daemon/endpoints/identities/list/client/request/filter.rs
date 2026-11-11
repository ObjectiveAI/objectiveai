//! What narrows a list of identities.

use serde::{Deserialize, Serialize};

/// The filter over the host's identities: every member optional, and
/// every one given a condition an identity passes or does not. In a
/// [list request](super::Frame) it is flattened into the request and
/// narrows what the daemon sends; in a grant it is how far the grant
/// reaches. A filter with no member given passes every identity.
///
/// # Any one of, every one of
///
/// A member that lists candidates matches an identity that is any one of
/// them. An empty list is absent, and matches every identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Filter {
    /// At any one of these issuers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub issuers: Vec<String>,
    /// Any one of these subjects, at whichever issuer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subjects: Vec<String>,
    /// Every one of these among the identity's tags, as
    /// [`tag`](crate::daemon::endpoints::identities::tag) put them.
    /// Absent when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_tags: Vec<String>,
    /// At least one of these among the identity's tags. Absent when
    /// empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_tags: Vec<String>,
}
