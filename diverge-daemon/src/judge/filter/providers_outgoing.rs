//! The outgoing providers filter, as a test.

use diverge_sdk::daemon::endpoints::providers::outgoing::list::client::request::Filter;

use crate::store::providers_outgoing::Outgoing;

/// Whether `provider` passes `filter`, given whether the daemon holds
/// a connection to it now.
pub fn test(filter: &Filter, provider: &Outgoing, connected: bool) -> bool {
    (filter.addresses.is_empty() || filter.addresses.contains(&provider.address))
        && (filter.kinds.is_empty() || filter.kinds.contains(&provider.kind()))
        && filter.connected.is_none_or(|wanted| connected == wanted)
        && (filter.creators.is_empty() || filter.creators.contains(&provider.creator))
        && filter.all_tags.iter().all(|tag| provider.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| provider.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| provider.created >= from)
        && filter.created_to.is_none_or(|to| provider.created <= to)
}
