//! The tools filter, as a test.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::tools::list::client::request::{Filter, Kind};
use diverge_sdk::daemon::key;

use crate::containers::ToolRun;
use crate::store::tools::Tool;

/// What a tool is, as a filter and a grant see it: the one shape for
/// a record and for a dependency, so that both are listed, judged and
/// narrowed by one test. A record brings its row, whether it is
/// active now and the agents it is attached to; a dependency brings
/// its run, always active, attached to the one agent it was deployed
/// for, with no tags and no template id.
pub struct Facts<'a> {
    /// Its name, if any.
    pub name: Option<&'a str>,
    /// The template it was made from, for a created record.
    pub template: Option<&'a str>,
    /// Who made it.
    pub creator: &'a Creator,
    /// Which of the three it is.
    pub kind: Kind,
    /// Whether it is active now.
    pub active: bool,
    /// The agents it is attached to.
    pub attached: &'a [key::Agent],
    /// Its tags.
    pub tags: &'a [String],
    /// When it was made.
    pub created: DateTime<Utc>,
}

impl<'a> Facts<'a> {
    /// A record, given whether it is active now and the agents it is
    /// attached to.
    pub fn record(tool: &'a Tool, active: bool, attached: &'a [key::Agent]) -> Facts<'a> {
        Facts {
            name: tool.name.as_deref(),
            template: tool.template(),
            creator: &tool.creator,
            kind: if tool.is_connected() { Kind::Connected } else { Kind::Created },
            active,
            attached,
            tags: &tool.tags,
            created: tool.created,
        }
    }

    /// A dependency's run: `None` for a run that is a record's.
    pub fn dependency(run: &'a ToolRun) -> Option<Facts<'a>> {
        let dependency = run.dependency.as_ref()?;
        Some(Facts {
            name: Some(&dependency.name),
            template: None,
            creator: &run.sender,
            kind: Kind::Dependency,
            active: true,
            attached: std::slice::from_ref(&dependency.agent_key),
            tags: &[],
            created: dependency.started,
        })
    }
}

/// Whether the tool passes `filter`. The filter's `agents` is every
/// one of them: a tool passes when each agent named is among those
/// it is attached to, by name.
pub fn test(filter: &Filter, facts: &Facts<'_>) -> bool {
    (filter.names.is_empty() || facts.name.is_some_and(|name| filter.names.iter().any(|wanted| wanted == name)))
        && (filter.templates.is_empty() || facts.template.is_some_and(|template| filter.templates.iter().any(|wanted| wanted == template)))
        && (filter.creators.is_empty() || filter.creators.contains(facts.creator))
        && filter.kind.is_none_or(|wanted| facts.kind == wanted)
        && filter.active.is_none_or(|wanted| facts.active == wanted)
        && filter
            .agents
            .iter()
            .all(|wanted| facts.attached.iter().any(|agent| agent.name.as_ref() == Some(wanted)))
        && filter.all_tags.iter().all(|tag| facts.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| facts.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| facts.created >= from)
        && filter.created_to.is_none_or(|to| facts.created <= to)
}
