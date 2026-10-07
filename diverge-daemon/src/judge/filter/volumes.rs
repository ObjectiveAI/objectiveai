//! The volumes filter, as a test.

use diverge_sdk::daemon::endpoints::volumes::list::client::request::Filter;

use crate::volumes::Listed;

/// Whether the volume passes `filter`.
pub fn test(filter: &Filter, listed: &Listed) -> bool {
    (filter.providers.is_empty() || filter.providers.contains(&listed.provider))
        && (filter.names.is_empty() || filter.names.contains(&listed.volume.name))
        && (filter.modes.is_empty() || filter.modes.contains(&listed.volume.mode))
        && filter.mounted.is_none_or(|wanted| listed.mounted() == wanted)
        && filter.created_from.is_none_or(|from| listed.created() >= from)
        && filter.created_to.is_none_or(|to| listed.created() <= to)
}
