//! Judging requests over the host's identities.
//!
//! The shape of [`volumes`](super::volumes): a making action — a login
//! of an identity not on the host yet — is held or it is not, judged by
//! nothing else; an action over an identity on the host is allowed by a
//! grant that holds it and whose `within` reaches the identity — `any`,
//! or the identities list filter as [`filter::identities::test`] reads
//! it, over the identity as a list answers it, its tags included; a
//! tagging action by a grant that holds it, reaches the identity, and
//! covers every tag named.

use diverge_sdk::daemon::endpoints::identities::list::client::request::Filter;
use diverge_sdk::daemon::endpoints::identities::list::server::response::Identity;
use diverge_sdk::daemon::grant::identities::{Make, Over, Permission};
use diverge_sdk::daemon::grant::{Tagging, Within};

use super::Standing;
use super::filter;

/// Whether the standing may do the making `action`.
pub fn make(standing: &Standing, action: Make) -> bool {
    if standing.admin() {
        return true;
    }
    standing
        .identities()
        .any(|permission| matches!(permission, Permission::Make(makes) if makes.contains(&action)))
}

/// Whether the standing holds `action` over any identity at all.
pub fn holds(standing: &Standing, action: Over) -> bool {
    if standing.admin() {
        return true;
    }
    standing
        .identities()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to the identity.
pub fn over(standing: &Standing, action: Over, identity: &Identity) -> bool {
    if standing.admin() {
        return true;
    }
    standing.identities().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, identity),
        _ => false,
    })
}

/// Whether the standing holds the tagging `action` over any identity at
/// all.
pub fn holds_tagging(standing: &Standing, action: Tagging) -> bool {
    if standing.admin() {
        return true;
    }
    standing
        .identities()
        .any(|permission| matches!(permission, Permission::Tags { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may put `tags` on the identity, or take them
/// off, as `action` says: one grant must hold the action, reach the
/// identity, and name every one of the tags or `any`.
pub fn tagging(standing: &Standing, action: Tagging, identity: &Identity, tags: &[String]) -> bool {
    if standing.admin() {
        return true;
    }
    standing.identities().any(|permission| match permission {
        Permission::Tags {
            actions,
            within,
            tags: scope,
        } => actions.contains(&action) && reaches(within, identity) && covers(scope, tags),
        _ => false,
    })
}

/// Whether a grant's `within` reaches the identity.
fn reaches(within: &Within<Filter>, identity: &Identity) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::identities::test(filter, identity),
    }
}

/// Whether a grant's tag scope names every one of `tags`.
fn covers(scope: &Within<Vec<String>>, tags: &[String]) -> bool {
    match scope {
        Within::Any => true,
        Within::Only(allowed) => tags.iter().all(|tag| allowed.contains(tag)),
    }
}
