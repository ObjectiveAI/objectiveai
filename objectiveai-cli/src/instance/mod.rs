//! Per-agent-instance subprocess runner — the personality the cli
//! takes on when invoked as `objectiveai-cli instance`. Lives in the
//! same crate as the cli itself so it can use any shared helper, error
//! variant, or wire type directly.
//!
//! Configuration is delivered as a single JSON blob over an inherited
//! anonymous-pipe handshake — see [`handshake`]. The handshake also
//! serves as the self-referential gate: invoking `objectiveai-cli
//! instance` directly from a shell fails the gate because no pipe was
//! inherited.
//!
//! Output is a typed [`InstanceEmission`] stream. Each variant
//! serializes to one JSON-line on stdout (`InstanceEmission`'s serde
//! impls ARE the wire format that the parent's `crate::streaming`
//! deserializes back into [`crate::streaming::InstanceItem`]).

mod agent_hierarchies;
mod agent_registry;
mod agents;
pub(crate) mod api;
mod functions;
pub mod handshake;
pub(crate) mod mcp_server;
pub mod request;
mod streaming;

use std::pin::Pin;

use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

use crate::error::Error;

use self::request::InstanceEndpoint;

/// One emission from the instance subprocess runtime. This is the
/// wire shape between the instance subprocess's stdout and the
/// parent's `crate::streaming::run_subprocess` — every JSON-line the
/// instance writes serializes to one of these variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstanceEmission {
    /// One-shot — fires after the runtime mints the response id and
    /// the log writer is fully wired up. Always emitted before any
    /// `Chunk` items.
    LogStreamReady { log_stream_ready: String },
    /// One streaming chunk's typed JSON, per-endpoint shape.
    Chunk(serde_json::Value),
    /// A non-fatal runtime warning (e.g. degraded pipe bind).
    Warning { message: String },
    /// A fatal runtime error from the subprocess. The instance binary
    /// uses `objectiveai_sdk::cli::Error` for its own stdout error
    /// lines (via `main.rs::write_error_line`); flatten that shape
    /// here so the parent can capture and propagate the error rather
    /// than panicking on an unknown variant.
    Error {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        level: Option<objectiveai_sdk::cli::Level>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fatal: Option<bool>,
        message: serde_json::Value,
    },
}

type EmissionStream = Pin<Box<dyn Stream<Item = Result<InstanceEmission, Error>> + Send>>;

/// Subprocess entrypoint. Validates the handshake, deserializes the
/// request blob, then dispatches to one of the typed endpoint
/// functions. Each handler returns a typed stream of
/// [`InstanceEmission`]s.
pub async fn run(ctx: crate::context::Context) -> Result<EmissionStream, Error> {
    let request = handshake::read_request().map_err(Error::Instance)?;
    let mcp_server = mcp_server::spawn(ctx.clone());
    let http = request.http;
    let pipes = request.pipes;
    let stream: EmissionStream = match request.endpoint {
        InstanceEndpoint::AgentsSpawn(params) => {
            agents::spawn::execute(ctx, http, pipes, mcp_server, params).await?
        }
        InstanceEndpoint::FunctionsExecute(params) => {
            functions::execute::execute(ctx, http, pipes, mcp_server, params).await?
        }
    };
    Ok(stream)
}
