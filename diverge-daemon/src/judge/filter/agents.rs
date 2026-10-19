//! The agents filter, as a test.

use diverge_sdk::daemon::endpoints::agents::list::client::request::Filter;

use crate::store::agents::Agent;

/// Whether `agent` passes `filter`, given whether a loop runs in it
/// now.
pub fn test(filter: &Filter, agent: &Agent, active: bool) -> bool {
    (filter.names.is_empty() || agent.name.as_ref().is_some_and(|name| filter.names.contains(name)))
        && (filter.templates.is_empty() || filter.templates.contains(&agent.template))
        && (filter.creators.is_empty() || filter.creators.contains(&agent.creator))
        && filter.active.is_none_or(|wanted| active == wanted)
        && filter.all_tags.iter().all(|tag| agent.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| agent.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| agent.created >= from)
        && filter.created_to.is_none_or(|to| agent.created <= to)
}
