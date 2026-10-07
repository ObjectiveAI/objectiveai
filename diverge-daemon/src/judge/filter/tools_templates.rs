//! The tool templates filter, as a test.

use diverge_sdk::daemon::endpoints::tools::templates::list::client::request::Filter;

use crate::store::tools_templates::Record;

/// Whether `record` passes `filter`, given whether some tool was
/// made from it.
pub fn test(filter: &Filter, record: &Record, in_use: bool) -> bool {
    (filter.ids.is_empty() || filter.ids.contains(&record.id))
        && (filter.creators.is_empty() || filter.creators.contains(&record.creator))
        && filter.in_use.is_none_or(|wanted| in_use == wanted)
        && filter.all_tags.iter().all(|tag| record.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| record.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| record.created >= from)
        && filter.created_to.is_none_or(|to| record.created <= to)
}
