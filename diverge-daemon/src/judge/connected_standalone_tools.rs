//! Judging requests over connected tools.
//!
//! The shape of [`standalone_tools`](super::standalone_tools): the
//! making action — the create — is held or it is not, and the create
//! also needs `connect` over the remote daemon's record it names, which
//! is [`daemons`](super::daemons)' to judge; an action over a connected
//! tool is allowed by a grant that holds it and whose `within` reaches
//! the tool — `any`, or the connected tools list filter as
//! [`filter::connected_standalone_tools::test`] reads it, over the
//! [`Facts`] of the record, which the caller knows; a tagging action by
//! a grant that holds it, reaches the tool, and covers every tag named.
//! An attach and a detach also need `edit` over the agent, or `attach`
//! over the agent template, which are [`agents`](super::agents)' and
//! [`agents_templates`](super::agents_templates)' to judge.

use diverge_sdk::daemon::endpoints::connected_standalone_tools::list::client::request::Filter;
use diverge_sdk::daemon::grant::connected_standalone_tools::{Make, Over, Permission};
use diverge_sdk::daemon::grant::{Tagging, Within};

use super::Standing;
use super::filter::{self, connected_standalone_tools::Facts};

/// Whether the standing may make a connected tool the way `make` says.
pub fn make(standing: &Standing, make: Make) -> bool {
    if standing.admin() {
        return true;
    }
    standing
        .connected_standalone_tools()
        .any(|permission| matches!(permission, Permission::Make(makes) if makes.contains(&make)))
}

/// Whether the standing holds `action` over any connected tool at all.
pub fn holds(standing: &Standing, action: Over) -> bool {
    if standing.admin() {
        return true;
    }
    standing
        .connected_standalone_tools()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to the connected tool the facts
/// describe.
pub fn over(standing: &Standing, action: Over, facts: &Facts<'_>) -> bool {
    if standing.admin() {
        return true;
    }
    standing.connected_standalone_tools().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, facts),
        _ => false,
    })
}

/// Whether the standing holds the tagging `action` over any connected
/// tool at all.
pub fn holds_tagging(standing: &Standing, action: Tagging) -> bool {
    if standing.admin() {
        return true;
    }
    standing
        .connected_standalone_tools()
        .any(|permission| matches!(permission, Permission::Tags { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may put `tags` on the connected tool, or take
/// them off, as `action` says.
pub fn tagging(standing: &Standing, action: Tagging, facts: &Facts<'_>, tags: &[String]) -> bool {
    if standing.admin() {
        return true;
    }
    standing.connected_standalone_tools().any(|permission| match permission {
        Permission::Tags {
            actions,
            within,
            tags: scope,
        } => actions.contains(&action) && reaches(within, facts) && covers(scope, tags),
        _ => false,
    })
}

/// Whether a grant's `within` reaches the tool.
fn reaches(within: &Within<Filter>, facts: &Facts<'_>) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::connected_standalone_tools::test(filter, facts),
    }
}

/// Whether a grant's tag scope names every one of `tags`.
fn covers(scope: &Within<Vec<String>>, tags: &[String]) -> bool {
    match scope {
        Within::Any => true,
        Within::Only(allowed) => tags.iter().all(|tag| allowed.contains(tag)),
    }
}
