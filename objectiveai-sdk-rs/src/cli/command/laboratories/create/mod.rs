//! `laboratories create` — create + start a laboratory container (a podman
//! container the conduit dials as a client-side MCP server). Inputs name the
//! container per state (`--id`), pick its base image (`--image`), and add
//! repeatable bind mounts (`--mount host=…,container=…`) and environment
//! entries (`--env KEY=VALUE`). `--client` is required (a required arg-group,
//! leaving room to add `--server` later). The leaf echoes back what it created.

use std::str::FromStr;

use crate::cli::command::CommandRequest;
use crate::cli::command::path_ref::tokenize;

/// Default working directory new agents start in, when `--cwd` is omitted (and
/// `#[serde(default)]` for older clients that don't send the field).
fn default_cwd() -> String {
    "/".to_string()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.create.Request")]
pub struct Request {
    pub path_type: Path,
    pub kind: Kind,
    pub id: String,
    pub image: String,
    pub mounts: Vec<Mount>,
    pub env: Vec<EnvVar>,
    /// Default working directory new agents start in; defaults to `/`.
    #[serde(default = "default_cwd")]
    pub cwd: String,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.create.Path")]
pub enum Path {
    #[serde(rename = "laboratories/create")]
    LaboratoriesCreate,
}

/// Which side of the conduit the laboratory serves. Only `Client` (a
/// client-side MCP server the conduit dials) exists today; the tag-by-`by`
/// shape leaves room to add `Server` later.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.laboratories.create.Kind")]
pub enum Kind {
    #[schemars(title = "Client")]
    Client,
}

/// One host→container bind mount. CLI wire form is the docker-style
/// `host=…,container=…` (both keys required).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.create.Mount")]
pub struct Mount {
    pub host: String,
    pub container: String,
}

impl FromStr for Mount {
    type Err = String;
    /// Parse a `--mount` arg. Accepted keys: `host`, `container` (both
    /// required). Anything else is an unknown-key error.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut host: Option<String> = None;
        let mut container: Option<String> = None;
        for (k, v) in tokenize(s)? {
            match k {
                "host" => host = Some(v.to_string()),
                "container" => container = Some(v.to_string()),
                other => return Err(format!("unknown key: {other}")),
            }
        }
        match (host, container) {
            (Some(host), Some(container)) => Ok(Mount { host, container }),
            (None, _) => Err("host is required".to_string()),
            (_, None) => Err("container is required".to_string()),
        }
    }
}

/// One environment entry. CLI wire form is `KEY=VALUE`, split on the first
/// `=` so values may contain `=`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.create.EnvVar")]
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

impl FromStr for EnvVar {
    type Err = String;
    /// Parse an `--env` arg. Splits on the FIRST `=`; a missing `=` is an
    /// error.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once('=') {
            Some((key, value)) => Ok(EnvVar {
                key: key.to_string(),
                value: value.to_string(),
            }),
            None => Err(format!("expected KEY=VALUE, got: {s}")),
        }
    }
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Echo of the created laboratory.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.create.Response")]
pub struct Response {
    pub id: String,
    pub image: String,
    pub mounts: Vec<Mount>,
    pub env: Vec<EnvVar>,
    pub cwd: String,
}

#[derive(clap::Args)]
#[command(
    group(clap::ArgGroup::new("side").required(true).args(["client"])),
    group(clap::ArgGroup::new("id_required").required(true).args(["id"])),
    group(clap::ArgGroup::new("image_required").required(true).args(["image"])),
)]
pub struct Args {
    /// Create a client-side laboratory (an MCP server the conduit dials).
    /// Required (part of the `side` arg-group, which will gain `--server`).
    #[arg(long)]
    pub client: bool,
    /// Laboratory id — names the per-state container.
    #[arg(long)]
    pub id: Option<String>,
    /// Container image to base the laboratory on.
    #[arg(long)]
    pub image: Option<String>,
    /// Repeatable `--mount host=…,container=…` bind mount.
    #[arg(long = "mount")]
    pub mounts: Vec<String>,
    /// Repeatable `--env KEY=VALUE` environment entry.
    #[arg(long = "env")]
    pub env: Vec<String>,
    /// Default working directory new agents start in; defaults to `/`.
    #[arg(long)]
    pub cwd: Option<String>,
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
        let id = args.id.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse("id", "--id is required".to_string())
        })?;
        let image = args.image.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "image",
                "--image is required".to_string(),
            )
        })?;
        if !args.client {
            return Err(crate::cli::command::FromArgsError::path_parse(
                "client",
                "--client is required".to_string(),
            ));
        }
        let mounts = args
            .mounts
            .iter()
            .map(|s| {
                s.parse::<Mount>()
                    .map_err(|m| crate::cli::command::FromArgsError::path_parse("mount", m))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let env = args
            .env
            .iter()
            .map(|s| {
                s.parse::<EnvVar>()
                    .map_err(|m| crate::cli::command::FromArgsError::path_parse("env", m))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let cwd = args.cwd.unwrap_or_else(default_cwd);
        Ok(Self {
            path_type: Path::LaboratoriesCreate,
            kind: Kind::Client,
            id,
            image,
            mounts,
            env,
            cwd,
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
