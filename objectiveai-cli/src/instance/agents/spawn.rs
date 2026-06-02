//! `agents spawn` — open an agent completion stream, yield each
//! chunk + the `LogStreamReady` handshake + any warnings as typed
//! [`InstanceEmission`] items, manage per-agent named pipes, and
//! write coalesced log files to `${config_base_dir}/logs/`.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::error::Error;
use crate::instance::InstanceEmission;
use crate::instance::request::{HttpConfig, PipeConfig};
use crate::instance::streaming;

type EmissionStream = Pin<Box<dyn Stream<Item = Result<InstanceEmission, Error>> + Send>>;

pub async fn execute(
    http: HttpConfig,
    pipes: PipeConfig,
    params: AgentCompletionCreateParams,
) -> Result<EmissionStream, Error> {
    let config_base_dir = pipes.config_base_dir().to_path_buf();
    let pipes_root = pipes.pipes_root();
    let client = http.build_http_client().map_err(Error::Instance)?;
    let conduit = pipes.build_conduit();

    let (tx, rx) = mpsc::channel::<Result<InstanceEmission, Error>>(16);
    let registry = crate::instance::pipes::PipeRegistry::new(tx.clone());

    pipes
        .try_eager_acquire(&registry)
        .await
        .map_err(Error::Instance)?;

    let fs_client = crate::filesystem::Client::new(
        Some(config_base_dir),
        None::<String>,
        None::<String>,
    );
    let caller_agent_instance_hierarchy = http.objectiveai_agent_instance_hierarchy.clone();
    let log_writer = fs_client
        .write_agent_completion(&params)
        .map_err(|e| Error::Instance(format!(
            "failed to build agent-completion log writer: {e}"
        )))?
        .with_caller_agent_instance_hierarchy(caller_agent_instance_hierarchy.clone());

    let (stream, notifier) =
        objectiveai_sdk::agent::completions::create_agent_completion_streaming(
            &client,
            params,
            conduit.clone(),
        )
        .await
        .map_err(|e| Error::Instance(format!(
            "failed to open agent-completion stream: {e}"
        )))?;
    conduit.install_notifier(notifier.clone());

    let stream = Box::pin(stream);
    let conduit_for_drop = conduit.clone();

    tokio::spawn(async move {
        let result = streaming::run_chunk_loop::<_, AgentCompletionChunk, _, _>(
            stream,
            notifier,
            pipes_root,
            caller_agent_instance_hierarchy,
            log_writer,
            tx.clone(),
            |agg: &mut AgentCompletionChunk, chunk: &AgentCompletionChunk| agg.push(chunk),
            Some(Box::new(move |seen: &std::collections::HashSet<String>| {
                conduit_for_drop.select_response_ids(seen);
            })),
            registry,
        )
        .await;

        match result {
            Ok(consumed) => {
                if let Some(error) = consumed.aggregate.and_then(|a| a.error) {
                    let _ = tx
                        .send(Err(Error::Instance(format!(
                            "agent completion failed: {error:?}"
                        ))))
                        .await;
                }
            }
            Err(e) => {
                let _ = tx.send(Err(Error::Instance(e))).await;
            }
        }
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
}
