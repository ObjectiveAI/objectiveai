//! `laboratories create` — create + start a laboratory container (a podman
//! container the conduit dials as a client-side MCP server). Inputs name the
//! container per state (`--id`), pick its base image (`--image`), and add
//! repeatable bind mounts (`--mount host=…,container=…`) and environment
//! entries (`--env KEY=VALUE`). `--client` is required (a required arg-group,
//! leaving room to add `--server` later). The leaf echoes back what it created.

use std::str::FromStr;

use crate::cli::command::CommandRequest;
use crate::laboratories::{EnvVar, Mount};

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
    /// The base image, split (`registry` + `name` + tag XOR digest) —
    /// a joined reference string is never accepted, so unqualified
    /// short names are unrepresentable.
    pub image: crate::laboratories::LaboratoryImage,
    pub mounts: Vec<Mount>,
    pub env: Vec<EnvVar>,
    /// Default working directory new agents start in; defaults to `/`.
    #[serde(default = "default_cwd")]
    pub cwd: String,
    /// The EXACT machine id (as `laboratories list` reports it) whose
    /// laboratory host should own the container. Provided together
    /// with `machine_state` or not at all; neither ⇒ the current
    /// machine + the daemon's own state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine: Option<String>,
    /// The state (on `machine`) whose laboratory host should own the
    /// container. Paired with `machine` — both or neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine_state: Option<String>,
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
    // No variant-level `#[schemars(title = "...")]`: a single-variant enum
    // collapses and hoists the variant title to the schema's top-level
    // `title`, which the JS codegen then uses as the module path — a title
    // of "Client" would clobber `src/client.ts`. Let `rename` drive it.
    Client,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Echo of the created laboratory, from the owning host's reply.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.create.Response")]
pub struct Response {
    pub id: String,
    pub image: crate::laboratories::LaboratoryImage,
    pub mounts: Vec<Mount>,
    pub env: Vec<EnvVar>,
    pub cwd: String,
    /// Unix seconds when the container was created, from podman's own
    /// record on the owning host. `None` when the host didn't report
    /// it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub created_at: Option<i64>,
    /// The machine whose laboratory host owns the container.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine: Option<crate::machine::MachineIdentity>,
    /// The state (on that machine) whose laboratory host owns the
    /// container.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine_state: Option<String>,
}

#[derive(clap::Args)]
#[command(
    group(clap::ArgGroup::new("side").required(true).args(["client"])),
    group(clap::ArgGroup::new("id_required").required(true).args(["id"])),
    group(clap::ArgGroup::new("image_source").required(true).args(["registry", "image_inline"])),
)]
pub struct Args {
    /// Create a client-side laboratory (an MCP server the conduit dials).
    /// Required (part of the `side` arg-group, which will gain `--server`).
    #[arg(long)]
    pub client: bool,
    /// Laboratory id — names the per-state container.
    #[arg(long)]
    pub id: Option<String>,
    /// The image host — e.g. `docker.io`, `ghcr.io`,
    /// `registry.example.com:5000`. Must be unambiguously a registry
    /// (domain, host:port, or localhost) — short names are refused.
    /// Mutually exclusive with `--image-inline`.
    #[arg(long, requires = "name")]
    pub registry: Option<String>,
    /// The image repository path — e.g. `library/bash`. Requires
    /// `--registry`.
    #[arg(long, requires = "registry")]
    pub name: Option<String>,
    /// Image tag (mutually exclusive with `--digest`; requires
    /// `--registry`).
    #[arg(long, conflicts_with_all = ["digest", "image_inline"], requires = "registry")]
    pub tag: Option<String>,
    /// Image digest, `<algorithm>:<64 hex>` (mutually exclusive with
    /// `--tag`; requires `--registry`).
    #[arg(long, conflicts_with = "image_inline", requires = "registry")]
    pub digest: Option<String>,
    /// The image spec INLINE, as a JSON string literal of
    /// Containerfile content (quoted + escaped on the command line;
    /// e.g. `--image-inline "\"FROM docker.io/library/bash:latest\n\""`).
    /// The host builds it on every create against an empty context.
    /// Mutually exclusive with the `--registry` family.
    #[arg(long)]
    pub image_inline: Option<String>,
    /// Repeatable `--mount host=…,container=…` bind mount.
    #[arg(long = "mount")]
    pub mounts: Vec<String>,
    /// Repeatable `--env KEY=VALUE` environment entry.
    #[arg(long = "env")]
    pub env: Vec<String>,
    /// Default working directory new agents start in; defaults to `/`.
    #[arg(long)]
    pub cwd: Option<String>,
    /// The EXACT machine id (from `laboratories list`) whose host
    /// should own the container. Requires `--machine-state`; neither ⇒
    /// the current machine + the daemon's own state.
    #[arg(long, requires = "machine_state")]
    pub machine: Option<String>,
    /// The state (on `--machine`) whose host should own the container.
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
        let id = args.id.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse("id", "--id is required".to_string())
        })?;
        // The image source: --image-inline XOR the --registry family.
        // Clap enforces the coarse exclusivity; this is the friendly
        // authoritative pass.
        let image = match (args.image_inline, args.registry) {
            (Some(inline_json), None) => {
                // The ARGV value is a JSON string literal; the typed
                // Request holds the PLAIN decoded Containerfile text.
                let containerfile: String = serde_json::from_str(&inline_json)
                    .map_err(|e| {
                        crate::cli::command::FromArgsError::path_parse(
                            "image-inline",
                            format!(
                                "--image-inline must be a JSON string literal \
                                 (quoted + escaped): {e}"
                            ),
                        )
                    })?;
                crate::laboratories::LaboratoryImage::Inline(
                    crate::laboratories::InlineLaboratoryImage { containerfile },
                )
            }
            (None, Some(registry)) => {
                let name = args.name.ok_or_else(|| {
                    crate::cli::command::FromArgsError::path_parse(
                        "name",
                        "--name is required with --registry".to_string(),
                    )
                })?;
                let pin = match (args.tag, args.digest) {
                    (Some(tag), None) => {
                        crate::laboratories::LaboratoryImagePin::Tag(tag)
                    }
                    (None, Some(digest)) => {
                        crate::laboratories::LaboratoryImagePin::Digest(digest)
                    }
                    _ => {
                        return Err(crate::cli::command::FromArgsError::path_parse(
                            "image",
                            "exactly one of --tag, --digest is required \
                             with --registry"
                                .to_string(),
                        ));
                    }
                };
                crate::laboratories::LaboratoryImage::Registry(
                    crate::laboratories::RegistryLaboratoryImage {
                        registry,
                        name,
                        pin,
                    },
                )
            }
            _ => {
                return Err(crate::cli::command::FromArgsError::path_parse(
                    "image",
                    "exactly one of --image-inline, --registry is required"
                        .to_string(),
                ));
            }
        };
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
        // Both-or-neither, re-validated beyond clap's mutual
        // `requires` (which only runs for argv-built requests).
        if args.machine.is_some() != args.machine_state.is_some() {
            return Err(crate::cli::command::FromArgsError::path_parse(
                "machine",
                "--machine and --machine-state must be provided together".to_string(),
            ));
        }
        Ok(Self {
            path_type: Path::LaboratoriesCreate,
            kind: Kind::Client,
            id,
            image,
            mounts,
            env,
            cwd,
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

/// One `/listen` broadcast run of `laboratories create`: the actual
/// [`Request`], the producer's
/// [`Identity`](crate::identity::Identity), and the
/// unary response future. See [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub struct ListenerExecution {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::daemon::command_listener::UnaryResponse<Response>,
}
