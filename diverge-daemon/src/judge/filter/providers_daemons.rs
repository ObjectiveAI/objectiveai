//! The daemons filter, as a test.

use diverge_sdk::daemon::endpoints::providers::daemons::list::client::request::Filter;

use crate::store::providers_daemons::Record;

/// Whether `record` passes `filter`, given whether this daemon holds a
/// connection to the daemon now. The filter's `providers` is any one
/// of them: a record passes when any link of its names one.
pub fn test(filter: &Filter, record: &Record, connected: bool) -> bool {
    (filter.names.is_empty() || filter.names.contains(&record.name))
        && (filter.providers.is_empty() || record.links.iter().any(|link| filter.providers.contains(&link.provider)))
        && filter.connected.is_none_or(|wanted| connected == wanted)
        && (filter.creators.is_empty() || filter.creators.contains(&record.creator))
        && filter.all_tags.iter().all(|tag| record.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| record.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| record.created >= from)
        && filter.created_to.is_none_or(|to| record.created <= to)
}
