//! Who is calling: an agent on a tool call, a person on a page's
//! request — each as the daemon attests it, never as the request says.

use axum::http::HeaderMap;
use diverge_sdk::shared::Identity;
use diverge_sdk::shared::mcp::attestation::{self, Caller};
use rmcp::model::MetaObject;
use serde::Serialize;

use crate::failure::Failure;

/// An agent, as the chat names one: by its template and its index,
/// which name it once and for all, with its name beside for reading.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
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
    /// The agent, as the daemon's endpoints name one.
    pub fn reference(&self) -> diverge_sdk::daemon::reference::Agent {
        diverge_sdk::daemon::reference::Agent::TemplateIndex { template: self.template.clone(), index: self.index as u64 }
    }
}

/// The agent calling a tool, from the call's `_meta`. A client over a
/// connect is refused: only an agent reads its messages and answers.
pub fn agent(meta: &MetaObject) -> Result<Participant, Failure> {
    match attestation::request(meta) {
        Ok(Caller::Agent(agent)) => Ok(Participant { template: agent.key.template, index: agent.key.index as i64, name: agent.key.name }),
        Ok(Caller::Client { identity, .. }) => Err(Failure::Caller(format!("{identity} is a client over a connect, not an agent"))),
        Err(error) => Err(Failure::Caller(error.to_string())),
    }
}

/// The person behind a page's request, from the `diverge.network`
/// header the daemon stamped it with: the account, and the scope the
/// request came on. A request with no stamp, one that does not read,
/// or one stamped as an agent, is refused.
pub fn person(headers: &HeaderMap) -> Result<(Identity, String), Failure> {
    let stamp = headers.get(diverge_sdk::shared::mcp::KEY).ok_or_else(|| Failure::Caller("the request carries no stamp".to_string()))?;
    let text = stamp.to_str().map_err(|error| Failure::Caller(error.to_string()))?;
    match serde_json::from_str::<Caller>(text) {
        Ok(Caller::Client { identity, scope }) => Ok((identity, scope)),
        Ok(Caller::Agent(_)) => Err(Failure::Caller("an agent, not a person".to_string())),
        Err(error) => Err(Failure::Caller(error.to_string())),
    }
}
