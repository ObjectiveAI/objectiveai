//! `agents wait` — block until an agent reaches the REQUESTED status:
//! exactly one of `--inactive` (the agent is done — the original
//! behavior, now explicit) or `--active` (the agent is up).
//!
//! Takes an instance hierarchy or a tag (the `agents enqueue`
//! selector shape — a plain ref has no live identity to wait on and
//! errors). The wait is uncapped either way — it blocks until the
//! target reaches the requested status, however long that takes, and
//! a target ALREADY in that status returns immediately.
//!
//! `--inactive`:
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
//! `--active`:
//! - **Instance**: resolves when the AIH holds its instance lock —
//!   the live-registry `Activated` edge; already-held returns
//!   immediately.
//! - **Tag**: a BOUND tag waits on its hierarchy as above; a GROUPED
//!   tag first waits for the tag to BIND (someone spawns it), then
//!   for the bound agent to be up.
//!
//! One use among many: a plugin daemon can gate its actions on its
//! agent being up by running `agents wait --active` before acting.
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
    /// The status to wait FOR: `true` resolves when the agent is
    /// ACTIVE (up — its instance lock held), `false` when it is
    /// INACTIVE (done — its instance lock free). Defaults to `false`
    /// on the wire, so a request predating this field keeps its
    /// original wait-until-done meaning.
    #[serde(default)]
    pub active: bool,
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
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Success-only: the wait completed — the target reached the
/// requested status.
pub type Response = crate::cli::command::Ok;

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("wait_mode")
        .required(true)
        .multiple(false)
        .args(["active", "inactive"])
))]
pub struct Args {
    #[command(flatten)]
    pub agent: crate::cli::command::agents::selector::AgentSelectorArgs,
    /// Resolve when the agent becomes ACTIVE — it is up, holding its
    /// instance lock. Mutually exclusive with `--inactive`; exactly
    /// one of the two is required.
    #[arg(long)]
    pub active: bool,
    /// Resolve when the agent is INACTIVE — done, its instance lock
    /// free (the former default behavior, now explicit). Mutually
    /// exclusive with `--active`; exactly one of the two is required.
    #[arg(long)]
    pub inactive: bool,
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
        let agent = AgentSelector::try_from(args.agent)?;
        Ok(Self {
            path_type: Path::AgentsWait,
            agent,
            // The `wait_mode` group guarantees exactly one of
            // `--active` / `--inactive`, so `active` is the full mode.
            active: args.active,
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

/// One `/listen` broadcast run of `agents wait`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// unary response future. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::broadcast_listener::UnaryResponse<Response>,
}
