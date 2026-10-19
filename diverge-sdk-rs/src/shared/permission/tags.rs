//! The one test a grant that names nothing reaches by.

use serde::{Deserialize, Serialize};

/// Which of a kind a grant reaches, by their tags: those carrying
/// every one of `all_tags` and any one of `any_tags`. A member that
/// is empty is absent, and passes everything; both empty is every one
/// of the kind, which `"any"` says more plainly. Read as a TEST, as
/// the daemon reads a `within`: a thing passes when it passes every
/// member given. The two words are the ones every list filter of the
/// daemon's already has, so a grant here and a grant of the daemon's
/// read tags the same way. No other member decodes: a `within` that
/// named something — `names`, `templates`, `creators` — would
/// otherwise read as no tags at all, which is everything, and a grant
/// must never widen by being misread.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Tags {
    /// Every one of these among the thing's tags. Absent when empty,
    /// and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_tags: Vec<String>,
    /// Any one of these among the thing's tags; with `all_tags`, both
    /// hold. Absent when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_tags: Vec<String>,
}
