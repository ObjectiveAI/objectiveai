//! `functions inventions recursive create alpha-scalar` — async handler stub.

use crate::cli::command::CommandRequest;
use crate::cli::command::agents::spawn::AgentSpec;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.inventions.recursive.create.alpha_scalar.Request")]
pub struct Request {
    pub path_type: Path,
    pub params: RequestParams,
    pub agent: AgentSpec,
    pub continuation: Option<String>,
    pub seed: Option<i64>,
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.inventions.recursive.create.alpha_scalar.Path")]
pub enum Path {
    #[serde(rename = "functions/inventions/recursive/create/alpha_scalar")]
    FunctionsInventionsRecursiveCreateAlphaScalar,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.inventions.recursive.create.alpha_scalar.RequestParams")]
pub struct RequestParams {
    pub name: String,
    pub spec: String,
    pub depth: u64,
    pub min_branch_width: u64,
    pub max_branch_width: u64,
    pub min_leaf_width: u64,
    pub max_leaf_width: u64,
}

impl RequestParams {
    fn push_flags(&self, out: &mut Vec<String>) {
        out.push("--name".to_string());
        out.push(self.name.clone());
        out.push("--spec".to_string());
        out.push(self.spec.clone());
        out.push("--depth".to_string());
        out.push(self.depth.to_string());
        out.push("--min-branch-width".to_string());
        out.push(self.min_branch_width.to_string());
        out.push("--max-branch-width".to_string());
        out.push(self.max_branch_width.to_string());
        out.push("--min-leaf-width".to_string());
        out.push(self.min_leaf_width.to_string());
        out.push("--max-leaf-width".to_string());
        out.push(self.max_leaf_width.to_string());
    }
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "functions".to_string(),
            "inventions".to_string(),
            "recursive".to_string(),
            "create".to_string(),
            "alpha-scalar".to_string(),
        ];
        self.params.push_flags(&mut argv);
        argv.push("--agent-inline".to_string());
        argv.push(serde_json::to_string(&self.agent).expect("agent serializes"));
        if let Some(c) = &self.continuation {
            argv.push("--continuation".to_string());
            argv.push(c.clone());
        }
        if let Some(seed) = self.seed {
            argv.push("--seed".to_string());
            argv.push(seed.to_string());
        }
        if let Some(advanced) = &self.dangerous_advanced {
            argv.push("--dangerous-advanced".to_string());
            argv.push(
                serde_json::to_string(advanced)
                    .expect("RequestDangerousAdvanced serializes"),
            );
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.inventions.recursive.create.alpha_scalar.RequestDangerousAdvanced")]
pub struct RequestDangerousAdvanced {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.functions.inventions.recursive.create.alpha_scalar.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "Chunk")]
    Chunk(crate::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk),
    #[schemars(title = "Id")]
    Id(String),
}

/// Non-chunk variant of [`ResponseItem`]. Returned by the unary `execute`
/// path (with `dangerous_advanced.stream` cleared) when the cli emits a
/// single bare id string.
pub type Response = String;

#[derive(clap::Args)]
pub struct Args {
    /// Function name.
    #[arg(long)]
    pub name: String,
    /// Specification/prompt for the invention.
    #[arg(long)]
    pub spec: String,
    /// Nesting depth (0 for leaf-only).
    #[arg(long)]
    pub depth: u64,
    /// Minimum branch width.
    #[arg(long)]
    pub min_branch_width: u64,
    /// Maximum branch width.
    #[arg(long)]
    pub max_branch_width: u64,
    /// Minimum leaf width.
    #[arg(long)]
    pub min_leaf_width: u64,
    /// Maximum leaf width.
    #[arg(long)]
    pub max_leaf_width: u64,
    /// Inline JSON agent definition.
    #[arg(long)]
    pub agent_inline: String,
    /// Continuation token from a previous response.
    #[arg(long)]
    pub continuation: Option<String>,
    /// Seed for deterministic mock responses.
    #[arg(long)]
    pub seed: Option<i64>,
    /// Advanced opt-in flags as inline JSON.
    #[arg(long)]
    pub dangerous_advanced: Option<String>,
    /// jq filter applied to the JSON output.
    #[arg(long)]
    pub jq: Option<String>,
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
        let agent = {
            let mut de = serde_json::Deserializer::from_str(&args.agent_inline);
            serde_path_to_error::deserialize(&mut de).map_err(|source| {
                crate::cli::command::FromArgsError {
                    field: "agent_inline",
                    source: source.into(),
                }
            })?
        };
        let dangerous_advanced = if let Some(s) = args.dangerous_advanced {
            let mut de = serde_json::Deserializer::from_str(&s);
            let v = serde_path_to_error::deserialize(&mut de).map_err(|source| {
                crate::cli::command::FromArgsError {
                    field: "dangerous_advanced",
                    source: source.into(),
                }
            })?;
            Some(v)
        } else {
            None
        };
        Ok(Self { path_type: Path::FunctionsInventionsRecursiveCreateAlphaScalar,
            params: RequestParams {
                name: args.name,
                spec: args.spec,
                depth: args.depth,
                min_branch_width: args.min_branch_width,
                max_branch_width: args.max_branch_width,
                min_leaf_width: args.min_leaf_width,
                max_leaf_width: args.max_leaf_width,
            },
            agent,
            continuation: args.continuation,
            seed: args.seed,
            dangerous_advanced,
            jq: args.jq,
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute_streaming<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.jq = None;
    let mut advanced = request.dangerous_advanced.unwrap_or_default();
    advanced.stream = Some(true);
    request.dangerous_advanced = Some(advanced);
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_streaming_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    jq: String,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.jq = Some(jq);
    let mut advanced = request.dangerous_advanced.unwrap_or_default();
    advanced.stream = Some(true);
    request.dangerous_advanced = Some(advanced);
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<Response, E::Error> {
    request.jq = None;
    if let Some(advanced) = request.dangerous_advanced.as_mut() {
        advanced.stream = None;
    }
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    jq: String,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<serde_json::Value, E::Error> {
    request.jq = Some(jq);
    if let Some(advanced) = request.dangerous_advanced.as_mut() {
        advanced.stream = None;
    }
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;


pub mod response_schema;
