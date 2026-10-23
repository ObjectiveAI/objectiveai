//! `agents`: whom the caller can write to.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use diverge_sdk::daemon::endpoints::agents::list::client::request::{Active, Filter, History};
use diverge_sdk::daemon::endpoints::agents::list::server::response::Agent;

use crate::courier::Call;
use crate::failure::Failure;

/// The tool's name.
pub const NAME: &str = "agents";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "List the agents you can write to that can read their mail: those that use this courier now, and those that \
have never run (they will use it once they start, if their image declares it). Each with its name, template and index.";

/// What `agents` takes: nothing.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {}

/// One agent, as the lister shows it.
#[derive(Debug, Serialize)]
struct Listed {
    /// Its name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Its template, by id.
    template: String,
    /// Its index among those made from the template.
    index: u64,
    /// Whether it uses this courier now: its last registration
    /// declared it.
    attached: bool,
    /// Whether it has never run.
    never_run: bool,
    /// Whether a loop runs in it now.
    active: bool,
}

/// The two lists, read at once: the agents whose last registration
/// declared this courier's template, and the agents that have never
/// run. Attached first, then never-run, each oldest first, each agent
/// once.
pub async fn call(call: Call<'_>, _parameters: Parameters) -> Result<Value, Failure> {
    let attached_filter = Filter { dependency_tool_templates: vec![call.template.to_string()], ..Default::default() };
    let never_filter = Filter { active: Some(Active::History(History::Never)), ..Default::default() };
    let (attached, never) = tokio::join!(
        call.state.daemon.list(&call.state.client, &attached_filter),
        call.state.daemon.list(&call.state.client, &never_filter),
    );
    let (attached, never) = (attached?, never?);
    let mut listed: Vec<Listed> = attached.iter().map(|agent| shown(agent, true)).collect();
    for agent in &never {
        if !listed.iter().any(|seen| seen.template == agent.template && seen.index == agent.index) {
            listed.push(shown(agent, false));
        }
    }
    Ok(json!({ "agents": listed }))
}

/// One agent as shown, attached or not.
fn shown(agent: &Agent, attached: bool) -> Listed {
    Listed {
        name: agent.name.clone(),
        template: agent.template.clone(),
        index: agent.index,
        attached,
        never_run: agent.last_active.is_none(),
        active: agent.active,
    }
}
