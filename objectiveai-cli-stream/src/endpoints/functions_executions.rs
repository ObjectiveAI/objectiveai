//! `functions executions` streaming endpoint.

use objectiveai_sdk::cli::output::Handle;
use objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams;
use objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk;

use crate::args::{BodySource, HttpArgs, PipeArgs};
use crate::streaming;

/// Run a function execution stream end-to-end.
///
/// 1. Build the HTTP client + MCP conduit from the parsed args.
/// 2. Open the streaming WS via the SDK.
/// 3. Hand off to [`streaming::run_chunk_loop`] which prints each
///    chunk as NDJSON, manages per-agent pipes, and accumulates.
/// 4. On stream end, surface any root-level execution error.
pub async fn run(
    http: &HttpArgs,
    pipes: &PipeArgs,
    body: BodySource,
    handle: &Handle,
) -> Result<(), String> {
    let params: FunctionExecutionCreateParams = body.resolve()?;
    let pipes_root = pipes.pipes_root()?;
    let client = http.build_http_client()?;
    let conduit = pipes.build_conduit();

    let (stream, notifier) =
        objectiveai_sdk::functions::executions::create_function_execution_streaming(
            &client, params, conduit,
        )
        .await
        .map_err(|e| format!("failed to open function-executions stream: {e}"))?;

    let stream = Box::pin(stream);

    let consumed = streaming::run_chunk_loop::<_, FunctionExecutionChunk, _>(
        stream,
        notifier,
        pipes_root,
        handle,
        |agg, chunk| agg.push(chunk),
    )
    .await
    .map_err(|e: objectiveai_sdk::HttpError| format!("stream error: {e}"))?;

    if let Some(error) = consumed.aggregate.and_then(|a| a.error) {
        return Err(format!("function execution failed: {error:?}"));
    }
    Ok(())
}
