//! The dependencies filter, as a test.

use diverge_sdk::daemon::endpoints::dependencies::list::client::request::{Filter, Type};
use diverge_sdk::daemon::endpoints::dependencies::list::server::response::Run;
use diverge_sdk::daemon::key;
use diverge_sdk::shared::containers::dependencies::Kind;

use crate::store::dependencies::Record;

/// What a dependency template is, as a filter and a grant see it: its
/// record, the agents that depend on it now, and the tools running
/// from it now.
pub struct Facts<'a> {
    /// The record.
    pub record: &'a Record,
    /// The agents whose last registration declared it, in order.
    pub agents: &'a [key::Agent],
    /// The tools running from it now.
    pub runs: &'a [Run],
}

/// Whether the template passes `filter`. The filter's `agents` is
/// every one of them: a template passes when each agent named is among
/// those that depend on it, by name. `depended` is whether any does;
/// `in_use` whether any tool runs from it.
pub fn test(filter: &Filter, facts: &Facts<'_>) -> bool {
    let record = facts.record;
    let (kind, scope) = match &record.template.kind {
        Kind::Transient { .. } => (Type::Transient, None),
        Kind::Service { scope } => (Type::Service, Some(*scope)),
    };
    (filter.ids.is_empty() || filter.ids.contains(&record.id))
        && (filter.creators.is_empty() || filter.creators.contains(&record.creator))
        && (filter.types.is_empty() || filter.types.contains(&kind))
        && (filter.scopes.is_empty() || scope.is_some_and(|scope| filter.scopes.contains(&scope)))
        && filter.depended.is_none_or(|wanted| !facts.agents.is_empty() == wanted)
        && filter
            .agents
            .iter()
            .all(|wanted| facts.agents.iter().any(|agent| agent.name.as_ref() == Some(wanted)))
        && filter.in_use.is_none_or(|wanted| !facts.runs.is_empty() == wanted)
        && filter.all_tags.iter().all(|tag| record.tags.contains(tag))
        && (filter.any_tags.is_empty() || filter.any_tags.iter().any(|tag| record.tags.contains(tag)))
        && filter.created_from.is_none_or(|from| record.created >= from)
        && filter.created_to.is_none_or(|to| record.created <= to)
}
