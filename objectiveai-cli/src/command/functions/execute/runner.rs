//! In-process driver for `functions execute`.
//!
//! Opens one upstream WebSocket via the SDK, drains chunks through
//! a coalescing log writer task, and yields a typed [`Event`]
//! stream straight back to the cli leaf. No subprocess, no
//! generics, no message-queue restart logic (that's spawn-specific).
//!
//! Per-chunk: claim a process-owned lock file for every
//! `agent_instance_hierarchy` referenced anywhere in the chunk's
//! nested task tree, via
//! [`crate::websockets::agent_hierarchies::ChunkAgentHierarchies`]
//! + [`crate::websockets::agent_registry::AgentInstanceRegistry`].
//!
//! Log writes go to a dedicated tokio task that coalesces bursts
//! via `mpsc::try_recv` — function executions emit many chunks
//! rapidly during a swiss-system round and we must not gate the
//! stream-consumption critical path on synchronous postgres writes.
//!
//! `LogStreamReady` equivalent: the writer task fires a oneshot
//! the first time `log_writer.primary_id()` returns Some. The main
//! loop reacts via `tokio::select!` so it doesn't sit blocked on
//! `stream.next()` while the writer races toward `finalize` —
//! important for single-chunk completions where the primary id
//! only lands during finalize.

use std::path::PathBuf;

use futures::Stream;
use futures::StreamExt;
use objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams;
use objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk;
use tokio::sync::mpsc;

use crate::context::Context;
use crate::db::logs::LogWriter;
use crate::error::Error;
use crate::websockets::agent_hierarchies::ChunkAgentHierarchies;
use crate::websockets::agent_registry::AgentInstanceRegistry;

/// Item yielded by [`run`]. The cli leaf maps it to its own typed
/// `ResponseItem` (`standard::ResponseItem` or
/// `swiss_system::ResponseItem`).
pub enum Event {
    /// One-shot. Fires after the writer task mints the primary
    /// log id. Always emitted before any `Chunk` item.
    Id(String),
    /// One chunk straight off the SDK stream.
    Chunk(FunctionExecutionChunk),
}

/// In-process driver. Builds the per-call WS infrastructure (MCP
/// server, conduit, log writer) and drives the chunk loop until
/// EOF. Yields `Event::Id` once + `Event::Chunk` per chunk in
/// order.
pub fn run(
    ctx: Context,
    params: FunctionExecutionCreateParams,
    agents_dir: PathBuf,
) -> impl Stream<Item = Result<Event, Error>> + Send {
    async_stream::try_stream! {
        let mut registry = AgentInstanceRegistry::new(agents_dir)
            .map_err(|e| Error::Instance(format!(
                "failed to open agent claim registry: {e}"
            )))?;

        // Per-call resources.
        let mcp_server = crate::websockets::mcp_server::spawn(ctx.clone());
        let conduit = crate::websockets::conduit::ConduitMcpHandler::new(
            mcp_server,
            ctx.clone(),
        );
        let log_writer = crate::db::logs::write_function_execution(
            &ctx.db, &params,
        )
        .map_err(|e| Error::Instance(format!(
            "failed to build function-execution log writer: {e}"
        )))?;

        let (sdk_stream, notifier) =
            objectiveai_sdk::functions::executions::create_function_execution_streaming(
                &ctx.http,
                params,
                conduit.clone(),
            )
            .await
            .map_err(|e| Error::Instance(format!(
                "failed to open function-execution stream: {e}"
            )))?;
        conduit.install_notifier(notifier);

        let mut sdk_stream = Box::pin(sdk_stream);

        // Coalesced log writes. Same pattern as the deleted
        // `instance::streaming::run_chunk_loop`: a dedicated task
        // drains chunks via mpsc, on every recv tries to drain any
        // more queued ones into a single coalesced write, and
        // signals primary_id via the oneshot the first time it
        // becomes available.
        let (tx, rx) = mpsc::unbounded_channel::<FunctionExecutionChunk>();
        let (log_ready_tx, log_ready_rx) =
            tokio::sync::oneshot::channel::<String>();
        let mut log_ready_rx = Some(log_ready_rx);
        let writer_task = tokio::spawn(writer_loop(rx, log_writer, log_ready_tx));

        // Local buffer for chunks observed before primary_id lands.
        // Drained the moment the oneshot fires.
        let mut buffered: Vec<FunctionExecutionChunk> = Vec::new();
        let mut id_emitted = false;
        let mut stream_err: Option<String> = None;

        loop {
            tokio::select! {
                biased;

                ready = async { log_ready_rx.as_mut().unwrap().await },
                    if !id_emitted && log_ready_rx.is_some() => {
                    log_ready_rx = None;
                    id_emitted = true;
                    if let Ok(id) = ready {
                        yield Event::Id(id);
                    }
                    for c in buffered.drain(..) {
                        yield Event::Chunk(c);
                    }
                }

                item = sdk_stream.next() => {
                    match item {
                        Some(Ok(chunk)) => {
                            // 0. Best-effort claim of every
                            //    agent_instance_hierarchy in the
                            //    chunk's nested task tree. Registry
                            //    HashMap dedupes; per-chunk dispatch
                            //    catches each fresh slot the moment
                            //    it appears on the wire.
                            for hier in chunk.agent_instance_hierarchies() {
                                registry.observe(hier);
                            }

                            // 1. Hand a clone to the writer task.
                            let _ = tx.send(chunk.clone());

                            // 2. Emit or buffer until Id fires.
                            if id_emitted {
                                yield Event::Chunk(chunk);
                            } else {
                                buffered.push(chunk);
                            }
                        }
                        Some(Err(e)) => {
                            stream_err = Some(format!("{e}"));
                            break;
                        }
                        None => break,
                    }
                }
            }
        }

        // EOF — close the writer's input channel so its `finalize`
        // runs. `finalize` flushes the one-behind chunk, which is
        // the only opportunity for primary_id on single-chunk
        // completions.
        drop(tx);

        if !id_emitted {
            if let Some(rx) = log_ready_rx.take() {
                if let Ok(id) = rx.await {
                    yield Event::Id(id);
                }
                for c in buffered.drain(..) {
                    yield Event::Chunk(c);
                }
            }
        }

        // Surface writer failures. Stream errors take precedence
        // since they're the upstream cause; writer errors are a
        // downstream symptom.
        let writer_outcome = writer_task.await.map_err(|e| {
            Error::Instance(format!("log writer task panicked: {e}"))
        })?;
        if let Some(e) = stream_err {
            Err(Error::Instance(e))?;
        }
        if let Err(e) = writer_outcome {
            Err(Error::Instance(format!("log writer failed: {e}")))?;
        }
    }
}

async fn writer_loop(
    mut rx: mpsc::UnboundedReceiver<FunctionExecutionChunk>,
    mut log_writer: LogWriter<FunctionExecutionChunk>,
    log_ready_tx: tokio::sync::oneshot::Sender<String>,
) -> Result<(), Error> {
    let mut agg: Option<FunctionExecutionChunk> = None;
    let mut log_ready_tx = Some(log_ready_tx);

    while let Some(first) = rx.recv().await {
        match &mut agg {
            Some(a) => a.push(&first),
            None => agg = Some(first),
        }
        // Coalesce any chunks already sitting in the channel.
        while let Ok(next) = rx.try_recv() {
            if let Some(a) = &mut agg {
                a.push(&next);
            }
        }
        if let Some(a) = &agg {
            log_writer.write(a).await?;
        }
        // First write where primary_id became available fires the
        // oneshot; subsequent loops are no-ops since `take` left
        // it None.
        if let Some(tx) = log_ready_tx.take() {
            if let Some(id) = log_writer.primary_id() {
                let _ = tx.send(id.to_string());
            } else {
                log_ready_tx = Some(tx);
            }
        }
    }

    log_writer.finalize().await?;

    // Last-chance fire — `finalize` flushed the one-behind chunk,
    // so this is the latest possible point primary_id can land
    // (single-chunk completions). If we still don't have one the
    // sender drops here and the receiver wakes `Err(Canceled)` in
    // `run()`.
    if let Some(tx) = log_ready_tx.take() {
        if let Some(id) = log_writer.primary_id() {
            let _ = tx.send(id.to_string());
        }
    }
    Ok(())
}
