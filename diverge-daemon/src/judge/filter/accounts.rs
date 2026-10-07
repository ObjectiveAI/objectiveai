//! The accounts filter, as a test.

use diverge_sdk::daemon::endpoints::accounts::list::client::request::Filter;

use crate::store::accounts::Account;

/// Whether `account` passes `filter`, given whether a client is
/// connected as it now.
pub fn test(filter: &Filter, account: &Account, connected: bool) -> bool {
    let named = account.name.as_deref();
    let identity = account.identity.as_deref();
    (filter.names.is_empty() || named.is_some_and(|name| filter.names.iter().any(|wanted| wanted == name)))
        && (filter.identities.is_empty()
            || identity.is_some_and(|identity| filter.identities.iter().any(|wanted| wanted == identity)))
        && filter.named.is_none_or(|wanted| named.is_some() == wanted)
        && filter.credentialed.is_none_or(|wanted| identity.is_some() == wanted)
        && (filter.roles.is_empty() || filter.roles.iter().any(|role| account.roles.contains(role)))
        && filter.connected.is_none_or(|wanted| connected == wanted)
        && (filter.creators.is_empty() || filter.creators.contains(&account.creator))
        && filter.all_tags.iter().all(|tag| account.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| account.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| account.created >= from)
        && filter.created_to.is_none_or(|to| account.created <= to)
}
