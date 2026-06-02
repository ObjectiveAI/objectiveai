//! `agents spawn` — open an agent completion stream,
//! emit each chunk as NDJSON to stdout, manage per-agent named pipes,
//! and write coalesced log files to `${config_base_dir}/logs/`.

use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk;
use objectiveai_sdk::cli::output::Handle;

use crate::instance::api::{BodySource, HttpArgs, PipeArgs};
use crate::instance::streaming;

pub async fn handle(
    http: &HttpArgs,
    pipes: &PipeArgs,
    body: BodySource,
    handle: &Handle,
) -> Result<(), String> {
    let params: AgentCompletionCreateParams = body.resolve()?;
    let config_base_dir = pipes.config_base_dir()?.to_path_buf();
    let pipes_root = pipes.pipes_root()?;
    let client = http.build_http_client()?;
    let conduit = pipes.build_conduit();

    // Shared between the eager admission probe (if --bind-agent-instance-hierarchy
    // is set) and the per-chunk binds inside `run_chunk_loop`. The
    // eager probe stashes its `Listener` here; the chunk loop's
    // first matching `ensure_pipe` consumes it.
    let registry = crate::instance::pipes::PipeRegistry::new();
    pipes.try_eager_acquire(&registry, handle).await?;

    let fs_client = objectiveai_sdk::filesystem::Client::new(
        Some(config_base_dir),
        None::<String>,
        None::<String>,
    );
    let caller_agent_instance_hierarchy = http.objectiveai_agent_instance_hierarchy.clone();
    let log_writer = fs_client
        .write_agent_completion(&params)
        .map_err(|e| format!("failed to build agent-completion log writer: {e}"))?
        .with_caller_agent_instance_hierarchy(caller_agent_instance_hierarchy.clone());

    let (stream, notifier) =
        objectiveai_sdk::agent::completions::create_agent_completion_streaming(
            &client,
            params,
            conduit.clone(),
        )
        .await
        .map_err(|e| format!("failed to open agent-completion stream: {e}"))?;
    conduit.install_notifier(notifier.clone());

    let stream = Box::pin(stream);

    // For every chunk that carries an agent response_id, ask the
    // conduit to evict that response_id's sibling losers. After the
    // first such chunk the winner's group is collapsed — subsequent
    // chunks for the same response_id are no-ops in the conduit's
    // group map. Generalizes cleanly to nested chunk trees.
    let conduit_for_drop = conduit.clone();
    let consumed = streaming::run_chunk_loop::<_, AgentCompletionChunk, _, _>(
        stream,
        notifier,
        pipes_root,
        caller_agent_instance_hierarchy,
        log_writer,
        handle,
        |agg: &mut AgentCompletionChunk, chunk: &AgentCompletionChunk| agg.push(chunk),
        Some(Box::new(move |seen: &std::collections::HashSet<String>| {
            conduit_for_drop.select_response_ids(seen);
        })),
        registry,
    )
    .await?;

    if let Some(error) = consumed.aggregate.and_then(|a| a.error) {
        return Err(format!("agent completion failed: {error:?}"));
    }
    Ok(())
}
