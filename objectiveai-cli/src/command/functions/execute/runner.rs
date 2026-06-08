//! In-process driver for `functions execute`.
//!
//! Opens one upstream WebSocket via the SDK, hands every chunk to
//! the [`LogWriter`] (which owns the coalescing listener task
//! internally), and yields a typed [`Event`] stream straight back to
//! the cli leaf. No subprocess, no generics, no message-queue
//! restart logic (that's spawn-specific).
//!
//! Per-chunk: claim a process-owned lock file for every
//! `agent_instance_hierarchy` referenced anywhere in the chunk's
//! nested task tree, via
//! [`crate::websockets::agent_hierarchies::ChunkAgentHierarchies`]
//! + [`crate::websockets::agent_registry::AgentInstanceRegistry`].
//!
//! `LogStreamReady` equivalent: the LogWriter exposes a
//! `oneshot::Receiver<String>` from its constructor that fires the
//! first time the listener task learns the stream's primary
//! `response_id`. The main loop reacts via `tokio::select!` so it
//! doesn't sit blocked on `stream.next()` while the writer races
//! toward `finalize` — important for single-chunk completions where
//! the primary id only lands after the SDK stream closes.

use std::path::PathBuf;

use futures::Stream;
use futures::StreamExt;
use objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams;
use objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk;

use crate::context::Context;
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
        // Function execution doesn't bind a tag — that's only the
        // `agents spawn --agent-tag` path. Pass `None` so
        // the conduit's read-message-queue handler skips the fused
        // tag-group upgrade.
        let conduit = crate::websockets::conduit::ConduitMcpHandler::new(
            mcp_server,
            ctx.clone(),
            None,
        );
        // The LogWriter owns a listener task internally; it
        // coalesces queued chunks and persists off this critical
        // path. The ready receiver fires the first time the
        // listener learns the primary response id.
        let (log_writer, log_ready_rx) = crate::db::logs::write_function_execution(
            &ctx.db, &params,
        )
        .map_err(|e| Error::Instance(format!(
            "failed to build function-execution log writer: {e}"
        )))?;
        let mut log_ready_rx = Some(log_ready_rx);

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

                            // 1. Hand a clone to the LogWriter. A
                            //    send error means the listener task
                            //    has exited (likely from an earlier
                            //    DB error) — treat it like a
                            //    stream-level failure.
                            if let Err(e) = log_writer.write(chunk.clone()) {
                                stream_err = Some(format!("{e}"));
                                break;
                            }

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

        // EOF — close the writer's input by consuming it via
        // `finalize`. The await blocks until the listener has
        // drained every queued chunk AND finished its in-flight
        // future, so both "queue empty" and "no work in flight"
        // hold by the time we proceed.
        let finalize_outcome = log_writer.finalize().await;

        // If the ready oneshot hadn't fired by stream EOF, the
        // listener may have just landed primary_id during its last
        // batch. Drain the receiver (now that the sender side has
        // been dropped by the finalize-consume path) so we can fire
        // a last-chance Event::Id for single-chunk completions.
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
        if let Some(e) = stream_err {
            Err(Error::Instance(e))?;
        }
        if let Err(e) = finalize_outcome {
            Err(Error::Instance(format!("log writer failed: {e}")))?;
        }
    }
}
