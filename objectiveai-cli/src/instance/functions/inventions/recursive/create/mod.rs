//! `functions inventions recursive create` — open a recursive
//! function-invention stream, emit each chunk as NDJSON to stdout,
//! manage per-agent named pipes, and write coalesced log files to
//! `${config_base_dir}/logs/`.

use objectiveai_sdk::cli::output::Handle;
use objectiveai_sdk::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams;
use objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk;

use crate::instance::request::{HttpConfig, PipeConfig};
use crate::instance::streaming;

pub async fn execute(
    http: &HttpConfig,
    pipes: &PipeConfig,
    params: FunctionInventionRecursiveCreateParams,
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
        .write_function_invention_recursive(&params)
        .map_err(|e| format!("failed to build function-invention-recursive log writer: {e}"))?
        .with_caller_agent_instance_hierarchy(caller_agent_instance_hierarchy.clone());

    let (stream, notifier) =
        objectiveai_sdk::functions::inventions::recursive::create_function_invention_recursive_streaming(
            &client, params, conduit.clone(),
        )
        .await
        .map_err(|e| format!("failed to open function-invention-recursive stream: {e}"))?;
    conduit.install_notifier(notifier.clone());

    let stream = Box::pin(stream);

    let conduit_for_drop = conduit.clone();
    let _consumed = streaming::run_chunk_loop::<_, FunctionInventionRecursiveChunk, _, _>(
        stream,
        notifier,
        pipes_root,
        caller_agent_instance_hierarchy,
        log_writer,
        handle,
        |agg: &mut FunctionInventionRecursiveChunk, chunk: &FunctionInventionRecursiveChunk| {
            agg.push(chunk)
        },
        Some(Box::new(move |seen: &std::collections::HashSet<String>| {
            conduit_for_drop.select_response_ids(seen);
        })),
        registry,
    )
    .await?;

    Ok(())
}
