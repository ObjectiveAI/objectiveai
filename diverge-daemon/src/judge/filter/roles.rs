//! The roles filter, as a test.

use diverge_sdk::daemon::endpoints::accounts::Reference;
use diverge_sdk::daemon::endpoints::roles::list::client::request::Filter;

use crate::store::roles::{Holder, Role};

/// Whether `role` passes `filter`.
pub fn test(filter: &Filter, role: &Role) -> bool {
    (filter.names.is_empty() || filter.names.contains(&role.name))
        && (filter.accounts.is_empty()
            || filter
                .accounts
                .iter()
                .any(|reference| role.holders.iter().any(|holder| holds(holder, reference))))
        && (filter.creators.is_empty() || filter.creators.contains(&role.creator))
        && filter.all_tags.iter().all(|tag| role.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| role.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| role.created >= from)
        && filter.created_to.is_none_or(|to| role.created <= to)
}

/// Whether the reference names the holder: by its name, or by its
/// credential's identity, whichever the reference gives.
fn holds(holder: &Holder, reference: &Reference) -> bool {
    match reference {
        Reference::Name { name } => holder.name.as_deref() == Some(name.as_str()),
        Reference::Identity { identity } => holder.identity.as_deref() == Some(identity.as_str()),
    }
}
