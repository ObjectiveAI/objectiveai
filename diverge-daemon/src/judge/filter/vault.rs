//! The vault filter, as a test.

use diverge_sdk::daemon::endpoints::vault::list::client::request::Filter;

use crate::store::vault::Entry;

/// Whether `entry` passes `filter`.
pub fn test(filter: &Filter, entry: &Entry) -> bool {
    (filter.keys.is_empty() || filter.keys.contains(&entry.key))
        && (filter.creators.is_empty() || filter.creators.contains(&entry.creator))
        && filter.all_tags.iter().all(|tag| entry.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| entry.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| entry.created >= from)
        && filter.created_to.is_none_or(|to| entry.created <= to)
}
