//! The agents filter, as a test.

use diverge_sdk::daemon::endpoints::agents::list::client::request::Filter;

use crate::judge::AgentState;
use crate::store::agents::Agent;

/// Whether `agent` passes `filter`, given its state now: `active` reads
/// whether a loop runs in it, and whether one ever has; `deployed`
/// whether its container is up.
pub fn test(filter: &Filter, agent: &Agent, state: AgentState) -> bool {
    (filter.names.is_empty() || agent.name.as_ref().is_some_and(|name| filter.names.contains(name)))
        && (filter.templates.is_empty() || filter.templates.contains(&agent.template))
        && (filter.creators.is_empty() || filter.creators.contains(&agent.creator))
        && filter.active.is_none_or(|wanted| wanted.test(state.active, agent.last_active.is_some()))
        && filter.deployed.is_none_or(|wanted| state.deployed == wanted)
        && (filter.dependency_tool_templates.is_empty() || agent.dependencies.iter().any(|id| filter.dependency_tool_templates.contains(id)))
        && filter.all_tags.iter().all(|tag| agent.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| agent.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| agent.created >= from)
        && filter.created_to.is_none_or(|to| agent.created <= to)
}
