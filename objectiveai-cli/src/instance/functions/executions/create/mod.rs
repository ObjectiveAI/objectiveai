//! `functions executions create` — open a function execution stream,
//! emit each chunk as NDJSON to stdout, manage per-agent named pipes,
//! and write coalesced log files to `${config_base_dir}/logs/`.

use objectiveai_sdk::cli::output::Handle;
use objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams;
use objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk;

use crate::instance::request::{HttpConfig, PipeConfig};
use crate::instance::streaming;

pub async fn execute(
    http: &HttpConfig,
    pipes: &PipeConfig,
    params: FunctionExecutionCreateParams,
    handle: &Handle,
) -> Result<(), String> {
    let config_base_dir = pipes.config_base_dir().to_path_buf();
    let pipes_root = pipes.pipes_root();
    let client = http.build_http_client()?;
    let conduit = pipes.build_conduit();

    let registry = crate::instance::pipes::PipeRegistry::new();
    pipes.try_eager_acquire(&registry, handle).await?;

    let fs_client = crate::filesystem::Client::new(
        Some(config_base_dir),
        None::<String>,
        None::<String>,
    );
    let caller_agent_instance_hierarchy =
        http.objectiveai_agent_instance_hierarchy.clone();
    let log_writer = fs_client
        .write_function_execution(&params)
        .map_err(|e| format!("failed to build function-execution log writer: {e}"))?
        .with_caller_agent_instance_hierarchy(caller_agent_instance_hierarchy.clone());

    let (stream, notifier) =
        objectiveai_sdk::functions::executions::create_function_execution_streaming(
            &client,
            params,
            conduit.clone(),
        )
        .await
        .map_err(|e| format!("failed to open function-executions stream: {e}"))?;
    conduit.install_notifier(notifier.clone());

    let stream = Box::pin(stream);

    let conduit_for_drop = conduit.clone();
    let consumed = streaming::run_chunk_loop::<_, FunctionExecutionChunk, _, _>(
        stream,
        notifier,
        pipes_root,
        caller_agent_instance_hierarchy,
        log_writer,
        handle,
        |agg: &mut FunctionExecutionChunk, chunk: &FunctionExecutionChunk| agg.push(chunk),
        Some(Box::new(move |seen: &std::collections::HashSet<String>| {
            conduit_for_drop.select_response_ids(seen);
        })),
        registry,
    )
    .await?;

    if let Some(error) = consumed.aggregate.and_then(|a| a.error) {
        return Err(format!("function execution failed: {error:?}"));
    }
    Ok(())
}
