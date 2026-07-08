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
//! Event::Id handshake: gated on
//! [`crate::db::logs::LogWriter::written_once`]. Every chunk we
//! receive from the SDK is sent into the LogWriter and either
//! yielded immediately (if `written_once` already flipped) or
//! buffered. The first chunk on which `written_once` returns true
//! triggers the Id emission (the LogWriter's
//! `oneshot::Receiver<String>` is awaited inline — it's ready by
//! that point because the listener fires the watch immediately
//! before the ready oneshot) followed by a drain of the buffer.
//! After the SDK stream ends, if `written_once` is still false but
//! the buffer is non-empty, we `wait_written_once().await` and then
//! emit the deferred Id + buffer drain before finalizing.

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
    /// One-shot. Fires after the LogWriter has confirmed at least
    /// one successful batch persistence (`written_once`). Always
    /// emitted before any `Chunk` or `Hierarchy` item.
    Id(String),
    /// One unique agent instance hierarchy participating in this
    /// execution, announced exactly once — right after its instance
    /// lock is acquired (mirroring `agents spawn`'s announcement).
    Hierarchy(String),
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
) -> impl Stream<Item = Result<Event, Error>> + Send {
    async_stream::try_stream! {
        let mut registry =
            AgentInstanceRegistry::new(ctx.filesystem.state_dir(), ctx.agent_locks_arc());

        // Per-call resources.
        let mcp_server = crate::websockets::mcp_server::spawn(ctx.clone());
        // Function execution doesn't bind a tag — that's only the
        // `agents spawn --agent-tag` path. Pass `None` so
        // the conduit's read-message-queue handler skips the fused
        // tag-group upgrade.
        let mcp_timeout_ms = ctx.resolve_mcp_timeout_ms().await?;
        let backoff_max_elapsed_time_ms =
            ctx.resolve_backoff_max_elapsed_time_ms().await?;
        let conduit = crate::websockets::conduit::ConduitMcpHandler::new(
            mcp_server,
            ctx.clone(),
            None,
            mcp_timeout_ms,
            backoff_max_elapsed_time_ms,
        );
        // The LogWriter owns a listener task internally; it
        // coalesces queued chunks and persists off this critical
        // path. The ready receiver fires the first time the
        // listener learns the primary response id — by which point
        // `written_once` has also been flipped, so we never await
        // this oneshot before the gate opens.
        let (log_writer, log_ready_rx) = crate::db::logs::write_function_execution(
            ctx.db_client().await?,
            &params,
            ctx.config.agent_instance_hierarchy.clone(),
            // Live-conversation tee: this one writer streams every
            // nested agent's rows; the daemon routes per-frame by AIH.
            Some(crate::db::logs::ConversationTee::spawn(
                ctx.filesystem.state_dir(),
            )),
        )
        .map_err(|e| Error::Instance(format!(
            "failed to build function-execution log writer: {e}"
        )))?;
        // Wrap so we can `take()` the receiver exactly once when
        // the Id gate opens (whether mid-stream or post-stream).
        let mut log_ready_rx = Some(log_ready_rx);

        let (sdk_stream, notifier) =
            objectiveai_sdk::functions::executions::create_function_execution_streaming(
                ctx.api_client().await?,
                params,
                conduit.clone(),
            )
            .await
            .map_err(|e| Error::Instance(format!(
                "failed to open function-execution stream: {e}"
            )))?;
        conduit.install_notifier(notifier);

        let mut sdk_stream = Box::pin(sdk_stream);

        // Local buffer: events held back until `written_once` flips,
        // so the execution Id is always the stream's first item.
        let mut buffered: Vec<Event> = Vec::new();
        let mut id_emitted = false;
        // Hierarchies already announced — the registry dedupes the
        // LOCK, this set dedupes the YIELD: never the same hierarchy
        // item twice per execution.
        let mut announced: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut stream_err: Option<String> = None;

        while let Some(item) = sdk_stream.next().await {
            match item {
                Ok(chunk) => {
                    // 0. Walk every `(agent_instance_hierarchy,
                    //    continuation)` pair in the chunk's nested
                    //    tree. Claim the lock-file slot for each
                    //    hier (registry HashMap dedupes; per-chunk
                    //    dispatch catches each fresh slot the moment
                    //    it appears on the wire), and queue an
                    //    `agent_continuations` upsert for each leaf
                    //    that carries a continuation. The Vec is
                    //    awaited via `try_join_all` before any
                    //    downstream work — by the time the chunk
                    //    yields, the rows are persisted.
                    let mut continuation_upserts: Vec<_> = Vec::new();
                    let mut fresh_hierarchies: Vec<String> = Vec::new();
                    for (hier, continuation) in chunk.agent_instance_hierarchies() {
                        registry.observe(hier).await;
                        if announced.insert(hier.to_string()) {
                            fresh_hierarchies.push(hier.to_string());
                        }
                        if let Some(c) = continuation {
                            continuation_upserts.push(
                                crate::db::agent_continuations::upsert(ctx.db_client().await?, hier, c),
                            );
                        }
                    }
                    if let Err(e) =
                        futures::future::try_join_all(continuation_upserts).await
                    {
                        stream_err = Some(format!("agent_continuations upsert: {e}"));
                        break;
                    }

                    // 1. Hand a clone to the LogWriter. A send
                    //    error means the listener task has exited
                    //    (likely from an earlier DB error) — treat
                    //    it like a stream-level failure.
                    if let Err(e) = log_writer.write(chunk.clone()) {
                        stream_err = Some(format!("{e}"));
                        break;
                    }

                    // 2. Id gate: once the LogWriter confirms it
                    //    has persisted at least one batch, take the
                    //    ready oneshot (already fired by then) and
                    //    emit Id + drain the buffer.
                    if !id_emitted && log_writer.written_once() {
                        let rx = log_ready_rx
                            .take()
                            .expect("ready_rx taken exactly once via id_emitted gate");
                        let id = rx.await.map_err(|e| Error::Instance(
                            format!("log writer ready signal lost: {e}")
                        ))?;
                        yield Event::Id(id);
                        for event in buffered.drain(..) {
                            yield event;
                        }
                        id_emitted = true;
                    }

                    // 3. Emit or buffer — hierarchy announcements
                    //    first, then the chunk that carried them.
                    if id_emitted {
                        for hier in fresh_hierarchies.drain(..) {
                            yield Event::Hierarchy(hier);
                        }
                        yield Event::Chunk(chunk);
                    } else {
                        buffered.extend(
                            fresh_hierarchies.drain(..).map(Event::Hierarchy),
                        );
                        buffered.push(Event::Chunk(chunk));
                    }
                }
                Err(e) => {
                    stream_err = Some(format!("{e}"));
                    break;
                }
            }
        }

        // Post-stream: if `written_once` never flipped during the
        // loop but we DID send chunks to the LogWriter, wait for
        // the first batch to land, then emit the deferred Id +
        // buffer drain. (If the buffer is empty there's no Id to
        // emit — we just finalize and propagate.)
        if !id_emitted && !buffered.is_empty() {
            match log_writer.wait_written_once().await {
                Ok(()) => {
                    let rx = log_ready_rx
                        .take()
                        .expect("ready_rx taken exactly once via id_emitted gate");
                    match rx.await {
                        Ok(id) => yield Event::Id(id),
                        Err(e) => {
                            stream_err.get_or_insert_with(||
                                format!("log writer ready signal lost: {e}")
                            );
                        }
                    }
                    for event in buffered.drain(..) {
                        yield event;
                    }
                    id_emitted = true;
                }
                Err(e) => {
                    stream_err.get_or_insert_with(|| format!("log writer wait: {e}"));
                }
            }
        }

        // Finalize the LogWriter (consumes; drops the sender;
        // awaits the listener). By the time this returns: the
        // queue is empty AND no work is in flight. A mid-stream
        // failure rides in — post-drain, the writer logs it as an
        // `error` row for every nested agent completion that never
        // finished (no usage yet); cleanly-finished completions get
        // nothing.
        let finalize_outcome = log_writer
            .finalize_with_stream_error(
                stream_err
                    .as_ref()
                    .map(|e| Error::Instance(e.clone()).output_message()),
            )
            .await;

        // Surface errors. Stream errors take precedence (upstream
        // cause); writer errors are a downstream symptom.
        if let Some(e) = stream_err {
            Err(Error::Instance(e))?;
        }
        if let Err(e) = finalize_outcome {
            Err(Error::Instance(format!("log writer failed: {e}")))?;
        }
    }
}
