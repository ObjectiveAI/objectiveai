//! What a providers filter lets through.

use diverge_sdk::daemon::endpoints::providers::list::client::request::Filter;
use diverge_sdk::shared::Identity;

/// Whether the provider of `identity`, `connected` or not, passes
/// `filter`.
pub fn test(filter: &Filter, identity: &Identity, connected: bool) -> bool {
    (filter.identities.is_empty() || filter.identities.contains(identity)) && filter.connected.is_none_or(|wanted| connected == wanted)
}
