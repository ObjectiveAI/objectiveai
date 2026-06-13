//! `agents wait` — block until an agent is done.
//!
//! Takes an instance hierarchy or a tag (the `agents enqueue`
//! selector shape — a plain ref has no live identity to wait on and
//! errors) plus a humantime timeout that caps the WHOLE wait,
//! across both subscriptions on the tag route. The timeout rides
//! the [`RequestBase`] envelope (`--timeout`) but is REQUIRED for
//! this leaf — enforced at `TryFrom<Args>` time and re-checked by
//! the CLI handler for wire-built requests.
//!
//! [`RequestBase`]: crate::cli::command::RequestBase
//!
//! - **Instance**: subscribe to the AIH lock's release; a free lock
//!   returns immediately.
//! - **Un-upgraded (GROUPED) tag**: the tag lock's holder is the
//!   spawn materializing the tag. If nobody holds it, nothing is
//!   materializing it — return immediately. Otherwise subscribe to
//!   its release, re-resolve the tag (the spawn flow upgrades
//!   GROUPED→BOUND strictly before releasing the tag lock, so a
//!   still-GROUPED tag after release is a systemic invariant
//!   violation and errors fatally), then fall through to the
//!   instance wait on the freshly bound hierarchy.
//!
//! Success is the bare `"Ok"` sentinel either way.

use crate::cli::command::CommandRequest;
use crate::cli::command::agents::selector::AgentSelector;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.wait.Request")]
pub struct Request {
    pub path_type: Path,
    /// Who to wait on — an instance hierarchy or a tag. A plain ref
    /// has no live identity and errors.
    pub agent: AgentSelector,
    // Carries `timeout_seconds` — the wall-clock cap across the
    // WHOLE wait (both subscriptions on the tag route). REQUIRED
    // for this leaf; see the module doc.
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.wait.Path")]
pub enum Path {
    #[serde(rename = "agents/wait")]
    AgentsWait,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["agents".to_string(), "wait".to_string()];
        self.agent.push_flags(&mut argv);
        self.base.push_flags(&mut argv);
        argv
    }
}

/// Success-only: the wait completed inside the timeout.
pub type Response = crate::cli::command::Ok;

#[derive(clap::Args)]
pub struct Args {
    #[command(flatten)]
    pub agent: crate::cli::command::agents::selector::AgentSelectorArgs,
    #[command(flatten)]
    pub base: crate::cli::command::RequestBaseArgs,
}

#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Command {
    #[command(flatten)]
    pub args: Args,
    #[command(subcommand)]
    pub schema: Option<Schema>,
}

#[derive(clap::Subcommand)]
pub enum Schema {
    /// Emit the JSON Schema for this leaf's `Request` type and exit.
    RequestSchema(request_schema::Args),
    /// Emit the JSON Schema for this leaf's `Response` type and exit.
    ResponseSchema(response_schema::Args),
}

impl TryFrom<Args> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(args: Args) -> Result<Self, Self::Error> {
        // The timeout rides the shared envelope (which validates
        // format and `> 0`), but THIS leaf requires it — an
        // uncapped wait on a dead-but-locked agent never returns.
        if args.base.timeout.is_none() {
            return Err(crate::cli::command::FromArgsError {
                field: "timeout",
                source: "required for agents wait".to_string().into(),
            });
        }
        let agent = AgentSelector::try_from(args.agent)?;
        Ok(Self {
            path_type: Path::AgentsWait,
            agent,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<Response, E::Error> {
    request.base.clear_transform();
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<serde_json::Value, E::Error> {
    request.base.set_transform(transform);
    executor.execute_one(request, agent_arguments).await
}

pub mod request_schema;

pub mod response_schema;
