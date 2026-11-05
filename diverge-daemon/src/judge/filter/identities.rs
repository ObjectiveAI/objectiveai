//! The identities filter, as a test.

use diverge_sdk::daemon::endpoints::identities::list::client::request::Filter;
use diverge_sdk::daemon::endpoints::identities::list::server::response::Identity;

/// Whether the identity passes `filter`.
pub fn test(filter: &Filter, identity: &Identity) -> bool {
    (filter.issuers.is_empty() || filter.issuers.contains(&identity.issuer))
        && (filter.subjects.is_empty() || filter.subjects.contains(&identity.subject))
        && filter.all_tags.iter().all(|tag| identity.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| identity.tags.contains(tag)))
}
