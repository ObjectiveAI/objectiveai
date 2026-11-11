//! The connected tools filter, as a test.

use diverge_sdk::daemon::endpoints::connected_standalone_tools::list::client::request::Filter;
use diverge_sdk::daemon::key;

use crate::store::connected_standalone_tools::ConnectedTool;

/// What a connected tool is, as a filter and a grant see it: its row,
/// whether it is active now, whether its connect is held, the agents
/// it is attached to directly and the agent templates it is attached
/// to.
pub struct Facts<'a> {
    /// The row.
    pub tool: &'a ConnectedTool,
    /// Whether it is active now: a call in flight.
    pub active: bool,
    /// Whether it is deployed now: its connect held.
    pub deployed: bool,
    /// The agents it is attached to directly.
    pub attached_agent_instances: &'a [key::Agent],
    /// The agent templates it is attached to, by id.
    pub attached_agent_templates: &'a [String],
}

/// Whether the tool passes `filter`. The rules:
///
/// 1. `names`, `daemons` and `creators` are any one of them: a tool
///    passes whose name, whose `daemon` — the first of its route's
///    daemons — or whose creator is among them; an empty list passes
///    every tool.
/// 2. `attached_agent_instances` and `attached_agent_templates` are
///    every one of them: a tool passes when each agent named is among
///    those it is attached to directly, by name, and each template
///    named among those it is attached to, by id.
/// 3. `active` and `deployed`, when given, are what the tool is now.
/// 4. `all_tags` every one, `any_tags` any one, of the tool's tags.
/// 5. `created_from` and `created_to` bound its `created`, inclusive.
pub fn test(filter: &Filter, facts: &Facts<'_>) -> bool {
    let tool = facts.tool;
    (filter.names.is_empty() || tool.name.as_ref().is_some_and(|name| filter.names.contains(name)))
        && (filter.daemons.is_empty() || filter.daemons.contains(&tool.daemon))
        && (filter.creators.is_empty() || filter.creators.contains(&tool.creator))
        && filter.active.is_none_or(|wanted| facts.active == wanted)
        && filter.deployed.is_none_or(|wanted| facts.deployed == wanted)
        && filter
            .attached_agent_instances
            .iter()
            .all(|wanted| facts.attached_agent_instances.iter().any(|agent| agent.name.as_ref() == Some(wanted)))
        && filter
            .attached_agent_templates
            .iter()
            .all(|wanted| facts.attached_agent_templates.contains(wanted))
        && filter.all_tags.iter().all(|tag| tool.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| tool.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| tool.created >= from)
        && filter.created_to.is_none_or(|to| tool.created <= to)
}
