//! The resources filter, as a test.

use diverge_sdk::daemon::endpoints::resources::list::client::request::Filter;

use crate::store::resources::Record;

/// Whether `record` passes `filter`, given whether some container
/// mounts it.
pub fn test(filter: &Filter, record: &Record, in_use: bool) -> bool {
    (filter.ids.is_empty() || filter.ids.contains(&record.id))
        && (filter.kinds.is_empty() || filter.kinds.contains(&record.kind))
        && filter.in_use.is_none_or(|wanted| in_use == wanted)
        && (filter.creators.is_empty() || filter.creators.contains(&record.creator))
        && filter.all_tags.iter().all(|tag| record.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| record.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| record.created >= from)
        && filter.created_to.is_none_or(|to| record.created <= to)
}
