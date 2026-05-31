//! `functions inventions recursive create` — open a recursive
//! function-invention stream, emit each chunk as NDJSON to stdout,
//! manage per-agent named pipes, and write coalesced log files to
//! `${config_base_dir}/logs/`.

use objectiveai_sdk::cli::output::Handle;
use objectiveai_sdk::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams;
use objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk;

use crate::api::{BodySource, HttpArgs, PipeArgs};
use crate::streaming;

pub async fn handle(
    http: &HttpArgs,
    pipes: &PipeArgs,
    body: BodySource,
    handle: &Handle,
) -> Result<(), String> {
    let params: FunctionInventionRecursiveCreateParams = body.resolve()?;
    let config_base_dir = pipes.config_base_dir()?.to_path_buf();
    let pipes_root = pipes.pipes_root()?;
    let client = http.build_http_client()?;
    let conduit = pipes.build_conduit();

    let fs_client = objectiveai_sdk::filesystem::Client::new(
        Some(config_base_dir),
        None::<String>,
        None::<String>,
    );
    let caller_agent_id = http.objectiveai_agent_id.clone();
    let log_writer = fs_client
        .write_function_invention_recursive(&params)
        .map_err(|e| format!("failed to build function-invention-recursive log writer: {e}"))?
        .with_caller_agent_id(caller_agent_id.clone());

    let (stream, notifier) =
        objectiveai_sdk::functions::inventions::recursive::create_function_invention_recursive_streaming(
            &client, params, conduit.clone(),
        )
        .await
        .map_err(|e| format!("failed to open function-invention-recursive stream: {e}"))?;
    conduit.install_notifier(notifier.clone());

    let stream = Box::pin(stream);

    // `FunctionInventionRecursiveChunk` has no top-level `error`
    // field — only `inventions_errors: Option<bool>` (a flag) and
    // per-invention inner errors that ride out on the NDJSON chunk
    // stream. The fatal-error post-condition the other endpoints
    // surface doesn't apply here.
    let _consumed = streaming::run_chunk_loop::<_, FunctionInventionRecursiveChunk, _, _>(
        stream,
        notifier,
        pipes_root,
        caller_agent_id,
        log_writer,
        handle,
        |agg: &mut FunctionInventionRecursiveChunk, chunk: &FunctionInventionRecursiveChunk| {
            agg.push(chunk)
        },
    )
    .await?;

    Ok(())
}
