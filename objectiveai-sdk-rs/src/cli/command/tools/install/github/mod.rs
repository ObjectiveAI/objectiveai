//! `tools install github` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tools.install.github.Request")]
pub struct Request {
    pub path_type: Path,
    pub owner: String,
    pub repository: String,
    pub commit_sha: Option<String>,
    pub allow_untrusted: bool,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tools.install.github.Path")]
pub enum Path {
    #[serde(rename = "tools/install/github")]
    ToolsInstallGithub,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "tools".to_string(),
            "install".to_string(),
            "github".to_string(),
            "--owner".to_string(),
            self.owner.clone(),
            "--repository".to_string(),
            self.repository.clone(),
        ];
        if let Some(sha) = &self.commit_sha {
            argv.push("--commit-sha".to_string());
            argv.push(sha.clone());
        }
        if self.allow_untrusted {
            argv.push("--allow-untrusted".to_string());
        }
        self.base.push_flags(&mut argv);
        argv
    }

    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tools.install.github.Response")]
pub struct Response {
    pub installed: bool,
}

#[derive(clap::Args)]
pub struct Args {
    /// GitHub repository owner.
    #[arg(long)]
    pub owner: String,
    /// GitHub repository name.
    #[arg(long)]
    pub repository: String,
    /// Pin install to a specific commit.
    #[arg(long)]
    pub commit_sha: Option<String>,
    /// Permit installs from untrusted sources.
    #[arg(long)]
    pub allow_untrusted: bool,
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
        Ok(Self { path_type: Path::ToolsInstallGithub,
            owner: args.owner,
            repository: args.repository,
            commit_sha: args.commit_sha,
            allow_untrusted: args.allow_untrusted,
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

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;


pub mod response_schema;
