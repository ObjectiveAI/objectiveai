//! Which providers a grant reaches.

use serde::{Deserialize, Serialize};

use crate::daemon::agent_log::Identity;

/// Which of the daemon's providers a grant reaches: by identity, by
/// the tags on their records, or by both. Read as a TEST, as every
/// `within` is: a provider passes when it passes every member given.
/// The rules:
///
/// 1. **`identities`**: the provider is any one of these. Absent when
///    empty, and then any provider.
/// 2. **`all_tags`**: the provider's record carries every one of
///    these. Absent when empty, and then any tags.
/// 3. **`any_tags`**: the provider's record carries any one of these.
///    Absent when empty, and then any tags.
/// 4. **Together, all hold**: a reach naming identities and tags
///    reaches the named providers that also carry the tags.
/// 5. **No other member decodes**: a `within` that named something
///    else would otherwise read as nothing at all, which is every
///    provider, and a grant must never widen by being misread.
///
/// It is the `within` of every grant that acts over a provider or
/// through one: [`providers`](super), over the records;
/// [`providers_daemons`](crate::daemon::grant::providers_daemons),
/// over the daemons connected through them; and
/// [`images`](crate::daemon::grant::images), over the daemon's Stores
/// on them. A grant a tool's program declares names nobody, and
/// becomes one of these carrying its tags alone.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Reach {
    /// Any one of these providers: see [`Identity`]. Absent when
    /// empty, and then any provider.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identities: Vec<Identity>,
    /// Every one of these among the provider's record's tags. Absent
    /// when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_tags: Vec<String>,
    /// Any one of these among the provider's record's tags; with
    /// `all_tags`, both hold. Absent when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_tags: Vec<String>,
}

impl Reach {
    /// Whether the reach passes the provider `identity`, whose record
    /// carries `tags`: rules 1 to 4.
    pub fn reaches(&self, identity: &Identity, tags: &[String]) -> bool {
        (self.identities.is_empty() || self.identities.contains(identity))
            && self.all_tags.iter().all(|tag| tags.contains(tag))
            && (self.any_tags.is_empty() || self.any_tags.iter().any(|tag| tags.contains(tag)))
    }
}
