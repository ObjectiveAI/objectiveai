//! Judging requests over dependency templates.
//!
//! The shape of [`accounts`](super::accounts) less the making action,
//! which a dependency template has none of: an action over a template
//! is allowed by a grant that holds it and whose `within` reaches the
//! template — `any`, or the family's list filter as
//! [`filter::dependency_tools::test`] reads it, over the [`Facts`] of the
//! record, the agents that depend on it and the tools running from
//! it, which the caller knows; a tagging action by a grant that holds
//! it, reaches the template, and covers every tag named. An action
//! over a tool running from a template is judged over the template.

use diverge_sdk::daemon::endpoints::dependency_tools::list::client::request::Filter;
use diverge_sdk::daemon::grant::dependency_tools::{Over, Permission};
use diverge_sdk::daemon::grant::{Tagging, Within};

use super::Standing;
use super::filter::{self, dependency_tools::Facts};

/// Whether the standing holds `action` over any dependency template
/// at all.
pub fn holds(standing: &Standing, action: Over) -> bool {
    standing
        .dependency_tools()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to the template the facts
/// describe.
pub fn over(standing: &Standing, action: Over, facts: &Facts<'_>) -> bool {
    standing.dependency_tools().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, facts),
        Permission::Tags { .. } => false,
    })
}

/// Whether the standing holds the tagging `action` over any dependency
/// template at all.
pub fn holds_tagging(standing: &Standing, action: Tagging) -> bool {
    standing
        .dependency_tools()
        .any(|permission| matches!(permission, Permission::Tags { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may put `tags` on the template, or take them
/// off, as `action` says.
pub fn tagging(standing: &Standing, action: Tagging, facts: &Facts<'_>, tags: &[String]) -> bool {
    standing.dependency_tools().any(|permission| match permission {
        Permission::Tags {
            actions,
            within,
            tags: scope,
        } => actions.contains(&action) && reaches(within, facts) && covers(scope, tags),
        Permission::Over { .. } => false,
    })
}

/// Whether a grant's `within` reaches the template.
fn reaches(within: &Within<Filter>, facts: &Facts<'_>) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::dependency_tools::test(filter, facts),
    }
}

/// Whether a grant's tag scope names every one of `tags`.
fn covers(scope: &Within<Vec<String>>, tags: &[String]) -> bool {
    match scope {
        Within::Any => true,
        Within::Only(allowed) => tags.iter().all(|tag| allowed.contains(tag)),
    }
}
