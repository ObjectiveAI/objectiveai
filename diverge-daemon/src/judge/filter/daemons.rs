//! What a daemons filter lets through.

use diverge_sdk::daemon::endpoints::providers::daemons::list::client::request::Filter;

use crate::config::Remote;

/// Whether `remote`, `connected` or not, passes `filter`.
pub fn test(filter: &Filter, remote: &Remote, connected: bool) -> bool {
    (filter.names.is_empty() || filter.names.contains(&remote.name))
        && (filter.providers.is_empty() || remote.links.iter().any(|link| filter.providers.contains(&link.provider)))
        && filter.connected.is_none_or(|wanted| connected == wanted)
}
