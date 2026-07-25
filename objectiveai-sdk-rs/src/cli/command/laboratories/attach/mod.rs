//! `laboratories attach` — attach a laboratory id to an agent
//! target (a tag, or an instance hierarchy via PAIH + `--agent-instance`).
//! The attachment is keyed on the tag (for a tag target) or on the AIH
//! (for an instance target); see the CLI handler for the lock + insert.

use crate::cli::command::CommandRequest;
use crate::cli::command::agents::selector::{AgentSelector, AgentSelectorArgs};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.attach.Request")]
pub struct Request {
    pub path_type: Path,
    pub selector: AgentSelector,
    pub laboratory_id: String,
    /// The EXACT machine id whose laboratory host serves the
    /// laboratory. Provided together with `machine_state` or not at
    /// all; neither ⇒ the current machine + the daemon's own state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine: Option<String>,
    /// The state (on `machine`) whose laboratory host serves the
    /// laboratory. Paired with `machine` — both or neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine_state: Option<String>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.attach.Path")]
pub enum Path {
    #[serde(rename = "laboratories/attach")]
    LaboratoriesAttach,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Confirmation — attach succeeded; echoes the laboratory id that was
/// attached.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.attach.Response")]
pub struct Response {
    /// The laboratory id that was attached to the target.
    pub laboratory_id: String,
    /// The machine id of the laboratory host the attachment row
    /// records (the provided pair, or the auto-filled local one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine: Option<String>,
    /// The state the attachment row records, paired with `machine`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine_state: Option<String>,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("laboratory_id_required").required(true).args(["laboratory_id"])))]
pub struct Args {
    #[command(flatten)]
    pub selector: AgentSelectorArgs,
    /// Laboratory id to attach to the target agent.
    #[arg(long)]
    pub laboratory_id: Option<String>,
    /// The EXACT machine id whose host serves the laboratory.
    /// Requires `--machine-state`; neither ⇒ the current machine +
    /// the daemon's own state.
    #[arg(long, requires = "machine_state")]
    pub machine: Option<String>,
    /// The state (on `--machine`) whose host serves the laboratory.
    /// Requires `--machine` — both or neither.
    #[arg(long, requires = "machine")]
    pub machine_state: Option<String>,
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
        let selector = AgentSelector::try_from(args.selector)?;
        let laboratory_id = args.laboratory_id.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "laboratory_id",
                "--laboratory-id is required".to_string(),
            )
        })?;
        // Both-or-neither, re-validated beyond clap's mutual
        // `requires` (which only runs for argv-built requests).
        if args.machine.is_some() != args.machine_state.is_some() {
            return Err(crate::cli::command::FromArgsError::path_parse(
                "machine",
                "--machine and --machine-state must be provided together".to_string(),
            ));
        }
        Ok(Self {
            path_type: Path::LaboratoriesAttach,
            selector,
            laboratory_id,
            machine: args.machine,
            machine_state: args.machine_state,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    identity: Option<&crate::identity::Identity>,
) -> Result<Response, E::Error> {
    request.base.clear_transform();
    executor.execute_one(request, identity).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,
    identity: Option<&crate::identity::Identity>,
) -> Result<serde_json::Value, E::Error> {
    request.base.set_transform(transform);
    executor.execute_one(request, identity).await
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;

pub mod response_schema;

/// One `/listen` broadcast run of `laboratories attach`: the actual
/// [`Request`], the producer's
/// [`Identity`](crate::identity::Identity), and the
/// unary response future. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::cli::broadcast_listener::UnaryResponse<Response>,
}
