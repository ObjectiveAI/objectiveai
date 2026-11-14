//! The images filter, as a test.

use diverge_sdk::daemon::endpoints::images::list::client::request::Filter;
use diverge_sdk::daemon::endpoints::images::list::server::response::Image;

/// Whether the image passes `filter`.
pub fn test(filter: &Filter, image: &Image) -> bool {
    (filter.providers.is_empty() || filter.providers.contains(&image.provider))
        && (filter.digests.is_empty() || filter.digests.contains(&image.digest))
        && filter.created_from.is_none_or(|from| image.created >= from)
        && filter.created_to.is_none_or(|to| image.created <= to)
}
