//! `agents spawn` — open an agent completion stream,
//! emit each chunk as NDJSON to stdout, manage per-agent named pipes,
//! and write coalesced log files to `${config_base_dir}/logs/`.

use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk;
use objectiveai_sdk::cli::output::Handle;

use crate::api::{BodySource, HttpArgs, PipeArgs};
use crate::streaming;

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

    let fs_client = objectiveai_sdk::filesystem::Client::new(
        Some(config_base_dir),
        None::<String>,
        None::<String>,
    );
    let caller_agent_id = http.objectiveai_agent_id.clone();
    let log_writer = fs_client
        .write_agent_completion(&params)
        .map_err(|e| format!("failed to build agent-completion log writer: {e}"))?
        .with_caller_agent_id(caller_agent_id.clone());

    let (stream, notifier) =
        objectiveai_sdk::agent::completions::create_agent_completion_streaming(
            &client, params, conduit.clone(),
        )
        .await
        .map_err(|e| format!("failed to open agent-completion stream: {e}"))?;
    conduit.install_notifier(notifier.clone());

    let stream = Box::pin(stream);

    let consumed = streaming::run_chunk_loop::<_, AgentCompletionChunk, _, _>(
        stream,
        notifier,
        pipes_root,
        caller_agent_id,
        log_writer,
        handle,
        |agg: &mut AgentCompletionChunk, chunk: &AgentCompletionChunk| agg.push(chunk),
    )
    .await?;

    if let Some(error) = consumed.aggregate.and_then(|a| a.error) {
        return Err(format!("agent completion failed: {error:?}"));
    }
    Ok(())
}
