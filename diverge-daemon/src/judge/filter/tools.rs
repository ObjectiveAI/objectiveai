//! The tools filter, as a test.

use diverge_sdk::daemon::endpoints::tools::list::client::request::{Filter, Kind};
use diverge_sdk::daemon::key;

use crate::store::tools::Tool;

/// Whether `tool` passes `filter`, given whether it is active now and
/// the agents it is attached to. The filter's `agents` is every one
/// of them: a tool passes when each agent named is among those it is
/// attached to, by name.
pub fn test(filter: &Filter, tool: &Tool, active: bool, attached: &[key::Agent]) -> bool {
    let kind = if tool.is_connected() { Kind::Connected } else { Kind::Created };
    (filter.names.is_empty() || tool.name.as_ref().is_some_and(|name| filter.names.contains(name)))
        && (filter.templates.is_empty() || tool.template().is_some_and(|template| filter.templates.iter().any(|wanted| wanted == template)))
        && (filter.creators.is_empty() || filter.creators.contains(&tool.creator))
        && filter.kind.is_none_or(|wanted| kind == wanted)
        && filter.active.is_none_or(|wanted| active == wanted)
        && filter
            .agents
            .iter()
            .all(|wanted| attached.iter().any(|agent| agent.name.as_ref() == Some(wanted)))
        && filter.all_tags.iter().all(|tag| tool.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| tool.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| tool.created >= from)
        && filter.created_to.is_none_or(|to| tool.created <= to)
}
