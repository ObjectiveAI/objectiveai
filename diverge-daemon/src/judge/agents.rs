//! Judging requests over agents.
//!
//! The shape of [`accounts`](super::accounts): a making action — the
//! create — is held or it is not; an action over an agent is allowed
//! by a grant that holds it and whose `within` reaches the agent —
//! `any`, or the agents list filter as [`filter::agents::test`] reads
//! it, with whether a loop runs in the agent now, which the caller
//! knows; a tagging action by a grant that holds it, reaches the
//! agent, and covers every tag named.

use diverge_sdk::daemon::endpoints::agents::list::client::request::Filter;
use diverge_sdk::daemon::grant::agents::{Make, Over, Permission};
use diverge_sdk::daemon::grant::{Tagging, Within};

use super::Standing;
use super::filter;
use crate::store::agents::Agent;

/// Whether the standing may create an agent.
pub fn create(standing: &Standing) -> bool {
    standing
        .agents()
        .any(|permission| matches!(permission, Permission::Make(makes) if makes.contains(&Make::Create)))
}

/// Whether the standing holds `action` over any agent at all.
pub fn holds(standing: &Standing, action: Over) -> bool {
    standing
        .agents()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to `agent`, given whether a
/// loop runs in it now.
pub fn over(standing: &Standing, action: Over, agent: &Agent, active: bool) -> bool {
    standing.agents().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, agent, active),
        _ => false,
    })
}

/// Whether the standing holds the tagging `action` over any agent at
/// all.
pub fn holds_tagging(standing: &Standing, action: Tagging) -> bool {
    standing
        .agents()
        .any(|permission| matches!(permission, Permission::Tags { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may put `tags` on `agent`, or take them off,
/// as `action` says.
pub fn tagging(standing: &Standing, action: Tagging, agent: &Agent, active: bool, tags: &[String]) -> bool {
    standing.agents().any(|permission| match permission {
        Permission::Tags {
            actions,
            within,
            tags: scope,
        } => actions.contains(&action) && reaches(within, agent, active) && covers(scope, tags),
        _ => false,
    })
}

/// Whether a grant's `within` reaches the agent.
fn reaches(within: &Within<Filter>, agent: &Agent, active: bool) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::agents::test(filter, agent, active),
    }
}

/// Whether a grant's tag scope names every one of `tags`.
fn covers(scope: &Within<Vec<String>>, tags: &[String]) -> bool {
    match scope {
        Within::Any => true,
        Within::Only(allowed) => tags.iter().all(|tag| allowed.contains(tag)),
    }
}
