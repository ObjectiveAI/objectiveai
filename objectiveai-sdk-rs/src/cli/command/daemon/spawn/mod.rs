//! `daemon spawn` — start (or confirm) the per-state plugin daemon.
//!
//! Two modes, selected by `dangerous_advanced.foreground`:
//! - **launcher** (unset/false): probe the per-state daemon lock; if a
//!   daemon already holds it, return readiness immediately, otherwise
//!   re-exec this binary as `daemon spawn` with `foreground: true`
//!   detached and return once it is up.
//! - **foreground** (true): THIS process becomes the resident daemon —
//!   it acquires the lock, launches every `daemon: true` plugin as
//!   `<exec> daemon begin` (leashed), binds a per-plugin socket, emits
//!   one readiness item, then serves until any plugin exits.
//!
//! Streaming leaf: the foreground daemon yields the readiness item as
//! its first line (the launcher's handshake) and keeps the stream open
//! while it serves; the launcher yields exactly one readiness item.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.daemon.spawn.Request")]
pub struct Request {
    pub path_type: Path,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.daemon.spawn.Path")]
pub enum Path {
    #[serde(rename = "daemon/spawn")]
    DaemonSpawn,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.daemon.spawn.RequestDangerousAdvanced")]
pub struct RequestDangerousAdvanced {
    /// `Some(true)` → THIS process becomes the resident daemon. When
    /// unset/false, `daemon spawn` re-execs itself with this set and
    /// returns once the daemon is up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub foreground: Option<bool>,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// One daemon-spawn stream item. The foreground daemon emits this once
/// it is ready (readiness handshake) and the launcher emits it once the
/// daemon is confirmed up.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.daemon.spawn.ResponseItem")]
pub struct ResponseItem {
    pub ok: bool,
}

#[derive(clap::Args)]
pub struct Args {
    /// Raw JSON for `RequestDangerousAdvanced` (e.g. `{"foreground":true}`).
    #[arg(long)]
    pub dangerous_advanced: Option<String>,
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
        let dangerous_advanced: Option<RequestDangerousAdvanced> =
            if let Some(s) = args.dangerous_advanced {
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
        Ok(Self {
            path_type: Path::DaemonSpawn,
            dangerous_advanced,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    identity: Option<&crate::identity::Identity>,
) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.base.clear_transform();
    executor.execute(request, identity).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,
    identity: Option<&crate::identity::Identity>,
) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.base.set_transform(transform);
    executor.execute(request, identity).await
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;

pub mod response_schema;

/// One `/listen` broadcast run of `daemon spawn`: the actual
/// [`Request`], the producer's
/// [`Identity`](crate::identity::Identity), and the
/// response-item stream. See [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub struct ListenerExecution {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::daemon::command_listener::ResponseItemStream<ResponseItem>,
}
