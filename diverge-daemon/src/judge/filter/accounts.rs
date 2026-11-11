//! The accounts filter, as a test.

use diverge_sdk::daemon::endpoints::accounts::list::client::request::Filter;

use crate::store::accounts::Account;
use diverge_sdk::shared::Identity;

/// Whether `account` passes `filter`, given whether a client is
/// connected as it now.
pub fn test(filter: &Filter, account: &Account, connected: bool) -> bool {
    (filter.identities.is_empty() || filter.identities.contains(&account.identity))
        && (filter.roles.is_empty() || filter.roles.iter().any(|role| account.roles.contains(role)))
        && filter.connected.is_none_or(|wanted| connected == wanted)
        && (filter.creators.is_empty() || filter.creators.contains(&account.creator))
        && filter.all_tags.iter().all(|tag| account.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| account.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| account.created >= from)
        && filter.created_to.is_none_or(|to| account.created <= to)
}

/// Whether an administrator the configuration names, with no account of
/// its own, passes the filter: by its identity and whether a client is
/// connected as it. It has no roles, no tags, no creator and no time it
/// was made, so a filter that asks for any of them lets it through only
/// if it asks for none.
pub fn test_admin(filter: &Filter, identity: &Identity, connected: bool) -> bool {
    (filter.identities.is_empty() || filter.identities.contains(identity))
        && filter.roles.is_empty()
        && filter.connected.is_none_or(|wanted| connected == wanted)
        && filter.creators.is_empty()
        && filter.all_tags.is_empty()
        && filter.any_tags.is_empty()
        && filter.created_from.is_none()
        && filter.created_to.is_none()
}
