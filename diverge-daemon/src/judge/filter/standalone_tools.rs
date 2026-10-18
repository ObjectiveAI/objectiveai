//! The tools filter, as a test.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::standalone_tools::list::client::request::{Filter, Kind};
use diverge_sdk::daemon::endpoints::standalone_tools::templates::Mode;
use diverge_sdk::daemon::key;

use crate::store::tools::Tool;

/// What a tool is, as a filter and a grant see it: its row, whether
/// it is active now, the agents it is attached to directly and the
/// agent templates it is attached to. A dependency tool is no tool of
/// this family, and is judged by
/// [`dependency_tools`](super::dependency_tools) over its template.
pub struct Facts<'a> {
    /// Its name, if any.
    pub name: Option<&'a str>,
    /// The template it was made from, for a created record.
    pub template: Option<&'a str>,
    /// Who made it.
    pub creator: &'a Creator,
    /// Which of the two it is.
    pub kind: Kind,
    /// Its template's mode, for a created record; a connected tool
    /// runs by another daemon's template and has none here.
    pub mode: Option<Mode>,
    /// Whether it is active now.
    pub active: bool,
    /// The agents it is attached to directly.
    pub attached_agent_instances: &'a [key::Agent],
    /// The agent templates it is attached to, by id.
    pub attached_agent_templates: &'a [String],
    /// Its tags.
    pub tags: &'a [String],
    /// When it was made.
    pub created: DateTime<Utc>,
}

impl<'a> Facts<'a> {
    /// A record, given whether it is active now, the agents and the
    /// agent templates it is attached to, and its template's mode
    /// when it has one.
    pub fn record(tool: &'a Tool, active: bool, agents: &'a [key::Agent], templates: &'a [String], mode: Option<Mode>) -> Facts<'a> {
        Facts {
            name: tool.name.as_deref(),
            template: tool.template(),
            creator: &tool.creator,
            kind: if tool.is_connected() { Kind::Connected } else { Kind::Created },
            mode,
            active,
            attached_agent_instances: agents,
            attached_agent_templates: templates,
            tags: &tool.tags,
            created: tool.created,
        }
    }
}

/// Whether the tool passes `filter`. The filter's
/// `attached_agent_instances` and `attached_agent_templates` are
/// every one of them: a tool passes when each agent named is among
/// those it is attached to directly, by name, and each template named
/// among those it is attached to, by id.
pub fn test(filter: &Filter, facts: &Facts<'_>) -> bool {
    (filter.names.is_empty() || facts.name.is_some_and(|name| filter.names.iter().any(|wanted| wanted == name)))
        && (filter.templates.is_empty() || facts.template.is_some_and(|template| filter.templates.iter().any(|wanted| wanted == template)))
        && (filter.creators.is_empty() || filter.creators.contains(facts.creator))
        && filter.kind.is_none_or(|wanted| facts.kind == wanted)
        && (filter.modes.is_empty() || facts.mode.is_some_and(|mode| filter.modes.contains(&mode)))
        && filter.active.is_none_or(|wanted| facts.active == wanted)
        && filter
            .attached_agent_instances
            .iter()
            .all(|wanted| facts.attached_agent_instances.iter().any(|agent| agent.name.as_ref() == Some(wanted)))
        && filter
            .attached_agent_templates
            .iter()
            .all(|wanted| facts.attached_agent_templates.contains(wanted))
        && filter.all_tags.iter().all(|tag| facts.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| facts.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| facts.created >= from)
        && filter.created_to.is_none_or(|to| facts.created <= to)
}
