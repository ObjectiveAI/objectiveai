//! Judging requests over the vault.
//!
//! The shape of [`roles`](super::roles): the one making action — a
//! set of a key the vault does not hold — is held or it is not,
//! judged by nothing else; an action over a key is allowed by a grant
//! that holds it and whose `within` reaches the key — `any`, or the
//! vault list filter as [`filter::vault::test`] reads it; a tagging
//! action by a grant that holds it, reaches the key, and covers every
//! tag named. A key the vault does not hold yet — one a create or an
//! edit binds ahead of its set — is reached by a grant whose `within`
//! is `any` and by no other: see [`any`].

use diverge_sdk::daemon::endpoints::vault::list::client::request::Filter;
use diverge_sdk::daemon::grant::vault::{Make, Over, Permission};
use diverge_sdk::daemon::grant::{Tagging, Within};

use super::Standing;
use super::filter;
use crate::store::vault::Entry;

/// Whether the standing may set a key the vault does not hold.
pub fn make(standing: &Standing, action: Make) -> bool {
    standing
        .vault()
        .any(|permission| matches!(permission, Permission::Make(makes) if makes.contains(&action)))
}

/// Whether the standing holds `action` over any key at all.
pub fn holds(standing: &Standing, action: Over) -> bool {
    standing
        .vault()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to the key.
pub fn over(standing: &Standing, action: Over, entry: &Entry) -> bool {
    standing.vault().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, entry),
        _ => false,
    })
}

/// Whether the standing holds `action` over every key there is, or
/// could be: a grant of the action whose `within` is `any`. What a
/// key the vault does not hold yet is judged by.
pub fn any(standing: &Standing, action: Over) -> bool {
    standing.vault().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && within.is_any(),
        _ => false,
    })
}

/// Whether the standing holds the tagging `action` over any key at
/// all.
pub fn holds_tagging(standing: &Standing, action: Tagging) -> bool {
    standing
        .vault()
        .any(|permission| matches!(permission, Permission::Tags { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may put `tags` on the key, or take them off,
/// as `action` says: one grant must hold the action, reach the key,
/// and name every one of the tags or `any`.
pub fn tagging(standing: &Standing, action: Tagging, entry: &Entry, tags: &[String]) -> bool {
    standing.vault().any(|permission| match permission {
        Permission::Tags {
            actions,
            within,
            tags: scope,
        } => actions.contains(&action) && reaches(within, entry) && covers(scope, tags),
        _ => false,
    })
}

/// Whether the grant's reach takes in the key.
fn reaches(within: &Within<Filter>, entry: &Entry) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::vault::test(filter, entry),
    }
}

/// Whether the grant's tags take in every one named.
fn covers(scope: &Within<Vec<String>>, tags: &[String]) -> bool {
    match scope {
        Within::Any => true,
        Within::Only(covered) => tags.iter().all(|tag| covered.contains(tag)),
    }
}
