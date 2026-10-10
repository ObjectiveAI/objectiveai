//! The incoming credentials filter, as a test.

use diverge_sdk::daemon::endpoints::providers::incoming::list::client::request::Filter;

use crate::store::providers_incoming::Incoming;

/// Whether `credential` passes `filter`, given whether a provider is
/// connected through it now.
pub fn test(filter: &Filter, credential: &Incoming, connected: bool) -> bool {
    (filter.identities.is_empty() || filter.identities.contains(&credential.identity))
        && filter.connected.is_none_or(|wanted| connected == wanted)
        && (filter.creators.is_empty() || filter.creators.contains(&credential.creator))
        && filter.all_tags.iter().all(|tag| credential.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| credential.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| credential.created >= from)
        && filter.created_to.is_none_or(|to| credential.created <= to)
}
