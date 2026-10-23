//! Who is calling, as every alarm names the agent it wakes.

use diverge_sdk::shared::mcp::attestation::{self, Caller};
use rmcp::model::MetaObject;
use serde::Serialize;

use crate::failure::Failure;

/// An agent, as the alarm tool names one: by its template and its
/// index, which name it once and for all, with its name beside for
/// reading. A name is free again once its agent is deleted, so it is
/// never the key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Participant {
    /// The agent's template, by id.
    pub template: String,
    /// The agent's index among those made from the template.
    pub index: i64,
    /// The agent's name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Participant {
    /// The agent, as the daemon's endpoints name one: by its template
    /// and its index.
    pub fn reference(&self) -> diverge_sdk::daemon::reference::Agent {
        diverge_sdk::daemon::reference::Agent::TemplateIndex { template: self.template.clone(), index: self.index as u64 }
    }
}

/// The caller of a request, from its `_meta`: the agent the daemon
/// attested, never what the request says of itself. A request with no
/// attestation, one that does not read as a caller, or a client over
/// a connect rather than an agent, is refused: an alarm wakes an
/// agent, and only the agent that set it.
pub fn caller(meta: &MetaObject) -> Result<Participant, Failure> {
    match attestation::request(meta) {
        Ok(Caller::Agent(agent)) => Ok(Participant { template: agent.key.template, index: agent.key.index as i64, name: agent.key.name }),
        Ok(Caller::Unbrokered { identity }) => Err(Failure::Caller(format!("{identity} is a client over a connect, not an agent"))),
        Err(error) => Err(Failure::Caller(error.to_string())),
    }
}
