//! Judging requests over roles.
//!
//! The twin of [`accounts`](super::accounts), over roles: the same
//! four questions, with no live state to ask about. `Over::Grant` is
//! judged here like any other action — it is what naming a role in an
//! account's `roles` takes, over that role.

use diverge_sdk::daemon::grant::roles::{Make, Over, Permission};
use diverge_sdk::daemon::grant::{Tagging, Within};

use super::filter;
use super::Standing;
use crate::store::roles::Role;

/// Whether the standing may create a role.
pub fn create(standing: &Standing) -> bool {
    standing
        .roles()
        .any(|permission| matches!(permission, Permission::Make(makes) if makes.contains(&Make::Create)))
}

/// Whether the standing holds `action` over any role at all.
pub fn holds(standing: &Standing, action: Over) -> bool {
    standing
        .roles()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to `role`.
pub fn over(standing: &Standing, action: Over, role: &Role) -> bool {
    standing.roles().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, role),
        _ => false,
    })
}

/// Whether the standing holds the tagging `action` over any role at
/// all.
pub fn holds_tagging(standing: &Standing, action: Tagging) -> bool {
    standing
        .roles()
        .any(|permission| matches!(permission, Permission::Tags { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may put `tags` on `role`, or take them off, as
/// `action` says.
pub fn tagging(standing: &Standing, action: Tagging, role: &Role, tags: &[String]) -> bool {
    standing.roles().any(|permission| match permission {
        Permission::Tags {
            actions,
            within,
            tags: scope,
        } => actions.contains(&action) && reaches(within, role) && covers(scope, tags),
        _ => false,
    })
}

/// Whether a grant's `within` reaches the role.
fn reaches(within: &Within<diverge_sdk::daemon::endpoints::roles::list::client::request::Filter>, role: &Role) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::roles::test(filter, role),
    }
}

/// Whether a grant's tag scope names every one of `tags`.
fn covers(scope: &Within<Vec<String>>, tags: &[String]) -> bool {
    match scope {
        Within::Any => true,
        Within::Only(allowed) => tags.iter().all(|tag| allowed.contains(tag)),
    }
}
