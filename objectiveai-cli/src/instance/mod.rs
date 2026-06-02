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

mod agents;
pub(crate) mod api;
mod functions;
pub mod handshake;
mod pipes;
pub mod request;
mod streaming;

use objectiveai_sdk::cli::output::Handle;

use self::request::InstanceEndpoint;

/// Subprocess entrypoint. Validates the handshake, deserializes the
/// request blob, then dispatches to one of the typed endpoint
/// functions. Output goes to `handle` as today (NDJSON on stdout).
pub async fn run(handle: &Handle) -> Result<(), String> {
    let request = handshake::read_request()?;
    let http = request.http;
    let pipes = request.pipes;
    match request.endpoint {
        InstanceEndpoint::AgentsSpawn(params) => {
            agents::spawn::execute(&http, &pipes, params, handle).await
        }
        InstanceEndpoint::FunctionsExecutionsCreate(params) => {
            functions::executions::create::execute(&http, &pipes, params, handle).await
        }
        InstanceEndpoint::FunctionsInventionsRecursiveCreate(params) => {
            functions::inventions::recursive::create::execute(
                &http, &pipes, params, handle,
            )
            .await
        }
    }
}
