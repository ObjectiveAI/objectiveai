//! `functions execute` — open a function execution stream,
//! yield each chunk + `LogStreamReady` + any warnings as typed
//! [`InstanceEmission`] items, manage per-agent named pipes, write
//! coalesced log files to `${config_base_dir}/logs/`.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams;
use objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk;
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
    mcp_server: crate::mcp_server::McpServerHandle,
    params: FunctionExecutionCreateParams,
) -> Result<EmissionStream, Error> {
    let client = http.build_http_client().map_err(Error::Instance)?;
    let fs_client = ctx.filesystem.clone();
    let db = ctx.db.clone();
    let agents_dir = fs_client.base_dir().join("instances").join("agents");
    let conduit = pipes.build_conduit(ctx, mcp_server);

    let (tx, rx) = mpsc::channel::<Result<InstanceEmission, Error>>(16);

    let _ = fs_client;
    let log_writer = crate::db::logs::write_function_execution(&db, &params)
        .map_err(|e| Error::Instance(format!(
            "failed to build function-execution log writer: {e}"
        )))?;

    let (stream, notifier) =
        objectiveai_sdk::functions::executions::create_function_execution_streaming(
            &client,
            params,
            conduit.clone(),
        )
        .await
        .map_err(|e| Error::Instance(format!(
            "failed to open function-executions stream: {e}"
        )))?;
    conduit.install_notifier(notifier);

    let stream = Box::pin(stream);

    tokio::spawn(async move {
        let result = streaming::run_chunk_loop::<_, FunctionExecutionChunk, _, _>(
            stream,
            log_writer,
            tx.clone(),
            |agg: &mut FunctionExecutionChunk, chunk: &FunctionExecutionChunk| agg.push(chunk),
            agents_dir,
        )
        .await;

        match result {
            Ok(consumed) => {
                if let Some(error) = consumed.aggregate.and_then(|a| a.error) {
                    let _ = tx
                        .send(Err(Error::Instance(format!(
                            "function execution failed: {error:?}"
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
