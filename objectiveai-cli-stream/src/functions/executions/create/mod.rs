//! `functions executions create` — open a function execution stream,
//! emit each chunk as NDJSON to stdout, manage per-agent named pipes,
//! and write coalesced log files to `${config_base_dir}/logs/`.

use objectiveai_sdk::cli::output::Handle;
use objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams;
use objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk;

use crate::api::{BodySource, HttpArgs, PipeArgs};
use crate::streaming;

/// Run a function execution stream end-to-end.
///
/// 1. Build the HTTP client + MCP conduit from the parsed args.
/// 2. Build a `LogWriter<FunctionExecutionChunk>` rooted at
///    `${config_base_dir}/logs/` — same on-disk layout the regular
///    CLI produces.
/// 3. Open the streaming WS via the SDK.
/// 4. Hand off to [`streaming::run_chunk_loop`] which prints each
///    chunk as NDJSON, manages per-agent pipes, writes to the log on
///    a separate coalescing task, emits `LogStreamReady` once the
///    root log id is known, and accumulates.
/// 5. On stream end, surface any root-level execution error.
pub async fn handle(
    http: &HttpArgs,
    pipes: &PipeArgs,
    body: BodySource,
    handle: &Handle,
) -> Result<(), String> {
    let params: FunctionExecutionCreateParams = body.resolve()?;
    let config_base_dir = pipes.config_base_dir()?.to_path_buf();
    let pipes_root = pipes.pipes_root()?;
    let client = http.build_http_client()?;
    let conduit = pipes.build_conduit();

    // Build the on-disk log writer. `filesystem::Client::logs_dir()`
    // = `${base_dir}/logs`, so this lands at
    // `${config_base_dir}/logs/functions/executions/<fexc-id>/...` —
    // byte-identical to `objectiveai-cli functions executions create`.
    let fs_client = objectiveai_sdk::filesystem::Client::new(
        Some(config_base_dir),
        None::<String>,
        None::<String>,
    );
    let caller_agent_id = http.objectiveai_agent_id.clone();
    let log_writer = fs_client
        .write_function_execution(&params)
        .map_err(|e| format!("failed to build function-execution log writer: {e}"))?
        .with_caller_agent_id(caller_agent_id.clone());

    let (stream, notifier) =
        objectiveai_sdk::functions::executions::create_function_execution_streaming(
            &client, params, conduit.clone(),
        )
        .await
        .map_err(|e| format!("failed to open function-executions stream: {e}"))?;
    conduit.install_notifier(notifier.clone());

    let stream = Box::pin(stream);

    let consumed = streaming::run_chunk_loop::<_, FunctionExecutionChunk, _, _>(
        stream,
        notifier,
        pipes_root,
        caller_agent_id,
        log_writer,
        handle,
        |agg: &mut FunctionExecutionChunk, chunk: &FunctionExecutionChunk| agg.push(chunk),
    )
    .await?;

    if let Some(error) = consumed.aggregate.and_then(|a| a.error) {
        return Err(format!("function execution failed: {error:?}"));
    }
    Ok(())
}
