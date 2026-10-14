//! The dependency tools filter, as a test.

use diverge_sdk::daemon::endpoints::dependency_tools::list::client::request::{Filter, Mode as Which};
use diverge_sdk::daemon::endpoints::dependency_tools::list::server::response::{Attached, Run};
use diverge_sdk::daemon::key;
use diverge_sdk::daemon::reference;
use diverge_sdk::shared::containers::dependencies::Mode;

use crate::store::dependency_tools::DependencyTool;

/// What a dependency tool is, as a filter and a grant see it: its
/// row, whose it is, the agents attached to it now, and its run, if
/// its container is up.
pub struct Facts<'a> {
    /// The row.
    pub tool: &'a DependencyTool,
    /// Whose it is, as the wire names it.
    pub owner: &'a key::Owner,
    /// The agents whose last registration declared it, in order.
    pub attached: &'a [Attached],
    /// Its run, while its container is up.
    pub run: Option<&'a Run>,
}

/// Whether the tool passes `filter`. The filter's `agents` is every
/// one of them: a tool passes when each agent named is among those
/// attached to it, by name. `owners` is any one of them, an agent
/// reference matched by name or once and for all. `depended` is
/// whether any agent is attached; `in_use` whether the container
/// runs.
pub fn test(filter: &Filter, facts: &Facts<'_>) -> bool {
    let tool = facts.tool;
    let (mode, scope) = match &tool.declared.mode {
        Mode::Transient { .. } => (Which::Transient, None),
        Mode::Service { scope } => (Which::Service, Some(*scope)),
    };
    (filter.ids.is_empty() || filter.ids.contains(&tool.template))
        && (filter.owners.is_empty() || filter.owners.iter().any(|wanted| owned_by(facts.owner, wanted)))
        && (filter.creators.is_empty() || filter.creators.contains(&tool.creator))
        && (filter.modes.is_empty() || filter.modes.contains(&mode))
        && (filter.scopes.is_empty() || scope.is_some_and(|scope| filter.scopes.contains(&scope)))
        && filter.depended.is_none_or(|wanted| !facts.attached.is_empty() == wanted)
        && filter
            .agents
            .iter()
            .all(|wanted| facts.attached.iter().any(|attached| attached.agent.name.as_ref() == Some(wanted)))
        && filter.in_use.is_none_or(|wanted| facts.run.is_some() == wanted)
        && filter.all_tags.iter().all(|tag| tool.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| tool.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| tool.created >= from)
        && filter.created_to.is_none_or(|to| tool.created <= to)
}

/// Whether the owner is the one the reference names: an agent by its
/// name as it is called now or by its template and index, an agent
/// template by id, or nobody.
fn owned_by(owner: &key::Owner, reference: &reference::Owner) -> bool {
    match (owner, reference) {
        (key::Owner::Agent(agent), reference::Owner::Agent(reference::Agent::Name { name })) => agent.name.as_ref() == Some(name),
        (key::Owner::Agent(agent), reference::Owner::Agent(reference::Agent::TemplateIndex { template, index })) => {
            &agent.template == template && agent.index == *index
        }
        (key::Owner::AgentTemplate { template: owned }, reference::Owner::AgentTemplate { template }) => owned == template,
        (key::Owner::Global, reference::Owner::Global) => true,
        _ => false,
    }
}
