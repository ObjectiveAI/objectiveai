//! `functions inventions recursive create` — open a recursive
//! function-invention stream, yield each chunk + `LogStreamReady` +
//! any warnings as typed [`InstanceEmission`] items, manage per-agent
//! named pipes, write coalesced log files to
//! `${config_base_dir}/logs/`.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams;
use objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::error::Error;
use crate::instance::InstanceEmission;
use crate::instance::request::{HttpConfig, PipeConfig};
use crate::instance::streaming;

type EmissionStream = Pin<Box<dyn Stream<Item = Result<InstanceEmission, Error>> + Send>>;

pub async fn execute(
    ctx: crate::context::Context,
    http: HttpConfig,
    pipes: PipeConfig,
    mcp_server: crate::instance::mcp_server::McpServerHandle,
    params: FunctionInventionRecursiveCreateParams,
) -> Result<EmissionStream, Error> {
    let client = http.build_http_client().map_err(Error::Instance)?;
    let fs_client = ctx.filesystem.clone();
    let db = ctx.db.clone();
    let conduit = pipes.build_conduit(ctx, mcp_server);

    let (tx, rx) = mpsc::channel::<Result<InstanceEmission, Error>>(16);

    let caller_agent_instance_hierarchy = Some(http.objectiveai_agent_instance_hierarchy.clone());
    let log_writer = fs_client
        .write_function_invention_recursive(&db, &params)
        .map_err(|e| Error::Instance(format!(
            "failed to build function-invention-recursive log writer: {e}"
        )))?
        .with_caller_agent_instance_hierarchy(caller_agent_instance_hierarchy);

    let (stream, notifier) =
        objectiveai_sdk::functions::inventions::recursive::create_function_invention_recursive_streaming(
            &client, params, conduit.clone(),
        )
        .await
        .map_err(|e| Error::Instance(format!(
            "failed to open function-invention-recursive stream: {e}"
        )))?;
    conduit.install_notifier(notifier);

    let stream = Box::pin(stream);

    tokio::spawn(async move {
        let result = streaming::run_chunk_loop::<_, FunctionInventionRecursiveChunk, _, _>(
            stream,
            log_writer,
            tx.clone(),
            |agg: &mut FunctionInventionRecursiveChunk, chunk: &FunctionInventionRecursiveChunk| {
                agg.push(chunk)
            },
        )
        .await;

        if let Err(e) = result {
            let _ = tx.send(Err(Error::Instance(e))).await;
        }
        // `FunctionInventionRecursiveChunk` has no top-level error
        // field; per-invention errors ride out on the chunk stream.
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
}
