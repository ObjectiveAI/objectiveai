//! Shared chunk-consumption loop every endpoint reuses.
//!
//! Drains the WS chunk stream, prints each chunk as one NDJSON line
//! on stdout (matches the existing `objectiveai-cli`'s output
//! convention), ensures a per-agent pipe exists for every
//! `agent_completion_id` the chunk references, writes each chunk
//! to a [`LogWriter`] on a separate coalescing task (so log writes
//! don't gate stream consumption), accumulates chunks into a final
//! aggregate via the caller-supplied `push` closure, and emits a
//! one-shot [`LogStreamReady`] notification with the root log id as
//! soon as the first write completes. On stream end fires every
//! pipe canceller.
//!
//! ## Notifications
//!
//! The per-agent pipe readers (in [`crate::pipes`]) forward each
//! received `RichContent` line into the writer task via a side-channel
//! mpsc, in addition to the existing fan-out to the API server via
//! `notifier`. The writer task owns a local `Vec<PendingNotification>`:
//! each arrival immediately writes the corresponding log file under
//! `agents/completions/request/notifications/<id>_<idx>.json` and reserves a
//! DB index; the row itself goes into the queue and is flushed to the
//! db when the next tool-response chunk for that agent comes in (so
//! the notification's index naturally precedes the tool response's).
//! Anything still queued at stream end is flushed by
//! [`LogWriter::finalize`].
//!
//! ## Order on stdout
//!
//! 1. NDJSON `Notification`s wrapping each streaming chunk, in
//!    arrival order (one per chunk).
//! 2. Exactly one `Notification { value: LogStreamReady { ... } }`,
//!    emitted by the writer task after the first chunk has been
//!    written to disk. This lands interleaved with the chunk
//!    notifications — its exact position is non-deterministic but
//!    always after at least one chunk. Same ordering the regular CLI
//!    produces.
//! 3. Pipe `Error` notifications (Warn-level, non-fatal) if any.

use std::path::PathBuf;

use futures::{Stream, StreamExt};
use objectiveai_sdk::Notifier;
use objectiveai_sdk::agent::completions::message::RichContent;
use objectiveai_sdk::agent::completions::response::streaming::AgentCompletionIds;
use objectiveai_sdk::cli::output::{Handle, LogStreamReady, Notification, Output};
use objectiveai_sdk::filesystem::db::pending::PendingNotification;
use objectiveai_sdk::filesystem::logs::LogWriter;
use serde::Serialize;

use crate::pipes::PipeRegistry;

/// Outcome of consuming a stream: the accumulated chunk (None when
/// the stream produced zero items) + the count of chunks consumed.
pub struct Consumed<Chunk> {
    pub aggregate: Option<Chunk>,
    pub chunk_count: usize,
}

/// Drain `stream`, emit each chunk as NDJSON to `handle`'s stdout
/// destination, manage per-agent pipes, coalesce-write to the log,
/// emit `LogStreamReady` once, accumulate. On stream end (success or
/// first error), tears down every active pipe and waits for the
/// writer task to flush its final batch.
pub async fn run_chunk_loop<S, Chunk, E, F>(
    mut stream: S,
    notifier: Notifier,
    pipes_root: PathBuf,
    caller_agent_id: Option<String>,
    log_writer: LogWriter<Chunk>,
    handle: &Handle,
    push: F,
) -> Result<Consumed<Chunk>, String>
where
    S: Stream<Item = Result<Chunk, E>> + Unpin,
    Chunk: AgentCompletionIds + Serialize + Clone + Send + Sync + 'static,
    E: std::fmt::Display,
    F: Fn(&mut Chunk, &Chunk) + Clone + Send + 'static,
{
    let registry = PipeRegistry::new();
    let mut aggregate: Option<Chunk> = None;
    let mut chunk_count: usize = 0;

    // Spawn the coalescing writer task. Main loop sends each chunk
    // via the unbounded channel; the writer task batches them up
    // (merging via `push`), writes one batch at a time, and emits
    // `LogStreamReady` once the root log id is known. A separate
    // notification channel carries `RichContent` notifications from
    // the pipe readers into the writer task's local pending queue.
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Chunk>();
    // Tuple: (lineage_agent_id, response_id, content). Threading both
    // axes keeps the writer from having to re-derive `response_id`
    // from `agent_id` — see the LogReference doc comment for the rule.
    let (notif_tx, notif_rx) = tokio::sync::mpsc::unbounded_channel::<(
        String,
        String,
        RichContent,
    )>();
    let writer_push = push.clone();
    let writer_handle = handle.clone();
    let writer_task = tokio::spawn(async move {
        writer_loop(rx, notif_rx, log_writer, writer_push, writer_handle).await
    });

    let mut stream_err: Option<String> = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                // 1. Emit the chunk to stdout as one NDJSON line.
                emit_chunk(&chunk, handle).await;

                // 2. Ensure a pipe is bound for every agent id this
                //    chunk references. `ensure_pipe` is idempotent —
                //    repeat ids are no-ops. Lineage-stamp each raw
                //    chunk-emitted id with the caller prefix so the
                //    pipe path matches the `messages.agent_id` form
                //    the writer stores; slashes inside the caller
                //    (multi-segment callers like `cli/parent-X`)
                //    become real subdirs via `pipes_root.join(...)`.
                for raw in chunk.agent_completion_ids() {
                    let lineage_id = match &caller_agent_id {
                        Some(c) => format!("{c}/{raw}"),
                        None => raw.to_string(),
                    };
                    registry
                        .ensure_pipe(
                            &lineage_id,
                            raw,
                            &pipes_root,
                            notifier.clone(),
                            notif_tx.clone(),
                            handle,
                        )
                        .await;
                }

                // 3. Hand a clone to the writer task. Best-effort —
                //    if the writer aborted on a disk error, the
                //    channel is closed and we drop the send. The
                //    writer's error surfaces on join below.
                let _ = tx.send(chunk.clone());

                // 4. Accumulate main-side.
                match aggregate.as_mut() {
                    Some(acc) => push(acc, &chunk),
                    None => aggregate = Some(chunk),
                }
                chunk_count += 1;
            }
            Err(e) => {
                stream_err = Some(format!("{e}"));
                break;
            }
        }
    }

    // Close the writer channel — the writer task drains the final
    // batch and returns.
    drop(tx);

    // Tear down every pipe. Reader tasks wake from their
    // tokio::select! and unlink the FS entry. Once every listener
    // drops its `notif_tx` clone, the writer task's notif_rx will
    // start returning None and writer_loop can finalize.
    registry.shutdown();
    drop(notif_tx);

    // Collect the writer task's outcome. A JoinError means the
    // writer panicked; a writer Err means a disk write failed.
    let writer_outcome = writer_task
        .await
        .map_err(|e| format!("log writer task panicked: {e}"))?;
    if let Err(e) = writer_outcome {
        return Err(format!("log writer failed: {e}"));
    }

    if let Some(e) = stream_err {
        return Err(e);
    }
    Ok(Consumed {
        aggregate,
        chunk_count,
    })
}

/// Coalescing writer loop. Mirrors `objectiveai-cli/src/log_stream::writer_loop`.
///
/// Blocks on the next chunk OR notification via `tokio::select!`,
/// drains anything that piled up while we were blocked, merges the
/// batch into the running aggregate via `push`, writes once. On the
/// first successful write, emits a one-shot `LogStreamReady` carrying
/// the root log id.
///
/// Pending notifications live in a local `Vec<PendingNotification>`.
/// Each arrival immediately calls `log_writer.write_notification`
/// (writes the notif file and reserves a DB index); the resulting
/// handle is queued. When a chunk's tool-response row is later
/// written, the matching handles are passed back into `log_writer.write`
/// for db insertion. Anything still queued when both channels close
/// is flushed by `log_writer.finalize`.
async fn writer_loop<Chunk, F>(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<Chunk>,
    mut notif_rx: tokio::sync::mpsc::UnboundedReceiver<(
        String,
        String,
        RichContent,
    )>,
    mut log_writer: LogWriter<Chunk>,
    push: F,
    handle: Handle,
) -> Result<(), objectiveai_sdk::filesystem::Error>
where
    F: Fn(&mut Chunk, &Chunk),
    Chunk: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionIds + Clone,
{
    let mut agg: Option<Chunk> = None;
    let mut pending: Vec<PendingNotification> = Vec::new();
    let mut logged_id = false;
    let mut chunk_channel_open = true;
    let mut notif_channel_open = true;

    while chunk_channel_open || notif_channel_open {
        // Block on whichever side has something next. `biased` makes
        // chunks the default winner on tie so chunk processing —
        // which includes draining queued notifs into the db — stays
        // prompt under load.
        tokio::select! {
            biased;
            chunk = rx.recv(), if chunk_channel_open => {
                match chunk {
                    Some(first) => {
                        // Coalesce additional chunks waiting in the channel.
                        match &mut agg {
                            Some(a) => push(a, &first),
                            None => agg = Some(first),
                        }
                        while let Ok(next) = rx.try_recv() {
                            if let Some(a) = &mut agg {
                                push(a, &next);
                            }
                        }
                        // Drain any notifs that arrived in the same window.
                        while let Ok((aid, response_id, content)) = notif_rx.try_recv() {
                            let p = log_writer
                                .write_notification(&aid, &response_id, &content)
                                .await?;
                            pending.push(p);
                        }
                        if let Some(a) = &agg {
                            log_writer.write(a, &mut pending).await?;
                        }
                        if !logged_id {
                            if let Some(id) = log_writer.primary_id() {
                                emit_log_stream_ready(id, &handle).await;
                                logged_id = true;
                            }
                        }
                    }
                    None => {
                        chunk_channel_open = false;
                    }
                }
            }
            notif = notif_rx.recv(), if notif_channel_open => {
                match notif {
                    Some((aid, response_id, content)) => {
                        let p = log_writer
                            .write_notification(&aid, &response_id, &content)
                            .await?;
                        pending.push(p);
                    }
                    None => {
                        notif_channel_open = false;
                    }
                }
            }
        }
    }

    // Both channels closed — flush any remaining queued notifications.
    log_writer.finalize(&mut pending).await?;
    Ok(())
}

async fn emit_log_stream_ready(id: &str, handle: &Handle) {
    let out = Output::Notification(Notification {
        agent_id: None,
        value: (LogStreamReady {
            log_stream_ready: id.to_string(),
        }).into(),
    });
    out.emit(handle).await;
}

async fn emit_chunk<C: Serialize>(chunk: &C, handle: &Handle) {
    let line = match serde_json::to_string(chunk) {
        Ok(s) => s,
        Err(_) => return,
    };
    // Use the cli output Handle so destination (Stdout/Collect/...)
    // is consistent with the rest of the cli's output convention.
    // For chunks we wrap them in a Notification so they ride the
    // same NDJSON envelope every other cli output line uses.
    let value: serde_json::Value = serde_json::from_str(&line).unwrap_or(serde_json::Value::Null);
    let out = Output::Notification(Notification {
        agent_id: None,
        value: objectiveai_sdk::cli::output::NotificationValue::other(&value),
    });
    out.emit(handle).await;
}
