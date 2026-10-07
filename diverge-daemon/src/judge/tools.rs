//! Judging requests over tools.
//!
//! The shape of [`accounts`](super::accounts): a making action — the
//! create, the connect — is held or it is not; an action over a tool
//! is allowed by a grant that holds it and whose `within` reaches the
//! tool — `any`, or the tools list filter as [`filter::tools::test`]
//! reads it, with whether the tool is active now and which agents it
//! is attached to, which the caller knows; a tagging action by a
//! grant that holds it, reaches the tool, and covers every tag named.
//! An attach and a detach also need `edit` over the agent, which is
//! [`agents`](super::agents)' to judge.

use diverge_sdk::daemon::endpoints::tools::list::client::request::Filter;
use diverge_sdk::daemon::grant::tools::{Make, Over, Permission};
use diverge_sdk::daemon::grant::{Tagging, Within};
use diverge_sdk::daemon::key;

use super::Standing;
use super::filter;
use crate::store::tools::Tool;

/// Whether the standing may make a tool the way `make` says.
pub fn make(standing: &Standing, make: Make) -> bool {
    standing
        .tools()
        .any(|permission| matches!(permission, Permission::Make(makes) if makes.contains(&make)))
}

/// Whether the standing holds `action` over any tool at all.
pub fn holds(standing: &Standing, action: Over) -> bool {
    standing
        .tools()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to `tool`, given whether it
/// is active now and the agents it is attached to.
pub fn over(standing: &Standing, action: Over, tool: &Tool, active: bool, attached: &[key::Agent]) -> bool {
    standing.tools().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, tool, active, attached),
        _ => false,
    })
}

/// Whether the standing holds the tagging `action` over any tool at
/// all.
pub fn holds_tagging(standing: &Standing, action: Tagging) -> bool {
    standing
        .tools()
        .any(|permission| matches!(permission, Permission::Tags { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may put `tags` on `tool`, or take them off,
/// as `action` says.
pub fn tagging(standing: &Standing, action: Tagging, tool: &Tool, active: bool, attached: &[key::Agent], tags: &[String]) -> bool {
    standing.tools().any(|permission| match permission {
        Permission::Tags {
            actions,
            within,
            tags: scope,
        } => actions.contains(&action) && reaches(within, tool, active, attached) && covers(scope, tags),
        _ => false,
    })
}

/// Whether a grant's `within` reaches the tool.
fn reaches(within: &Within<Filter>, tool: &Tool, active: bool, attached: &[key::Agent]) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::tools::test(filter, tool, active, attached),
    }
}

/// Whether a grant's tag scope names every one of `tags`.
fn covers(scope: &Within<Vec<String>>, tags: &[String]) -> bool {
    match scope {
        Within::Any => true,
        Within::Only(allowed) => tags.iter().all(|tag| allowed.contains(tag)),
    }
}
