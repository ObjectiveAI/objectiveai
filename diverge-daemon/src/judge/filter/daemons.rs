//! The daemons filter, as a test.

use diverge_sdk::daemon::endpoints::daemons::Daemon;
use diverge_sdk::daemon::endpoints::daemons::list::client::request::Filter;

/// Whether the record, as a list reports it, passes `filter`.
pub fn test(filter: &Filter, daemon: &Daemon) -> bool {
    (filter.names.is_empty() || filter.names.contains(&daemon.name))
        && (filter.issuers.is_empty() || filter.issuers.contains(&daemon.daemon.issuer))
        && (filter.creators.is_empty() || filter.creators.contains(&daemon.creator))
        && filter.connected.is_none_or(|wanted| daemon.connected == wanted)
        && filter.all_tags.iter().all(|tag| daemon.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| daemon.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| daemon.created >= from)
        && filter.created_to.is_none_or(|to| daemon.created <= to)
}
