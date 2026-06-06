//! Shared chunk-consumption loop every endpoint reuses.
//!
//! Drains the WS chunk stream, sends each chunk as an
//! [`InstanceEmission::Chunk`] through `emissions_tx`, ensures a
//! per-agent pipe exists for every `agent_completion_id` the chunk
//! references, writes each chunk to a [`LogWriter`] on a separate
//! coalescing task (so log writes don't gate stream consumption),
//! accumulates chunks into a final aggregate via the caller-supplied
//! `push` closure, and emits a one-shot
//! [`InstanceEmission::LogStreamReady`] with the root log id as soon
//! as the first write completes. On stream end fires every pipe
//! canceller.

use std::path::PathBuf;

use futures::{Stream, StreamExt};
use objectiveai_sdk::Notifier;
use objectiveai_sdk::agent::completions::message::RichContent;
use objectiveai_sdk::agent::completions::response::streaming::AgentCompletionIds;
use objectiveai_sdk::cli::command::agents::read::subscribe::RequestMessageKind;
use serde::Serialize;
use tokio::sync::mpsc;

use crate::error::Error;
use crate::filesystem::db::pending::PendingNotification;
use crate::filesystem::logs::{LogWriter, SubscribeEvent};
use crate::instance::InstanceEmission;
use crate::instance::pipes::{BindStatus, PipeRegistry};

pub type EmissionsTx = mpsc::Sender<Result<InstanceEmission, Error>>;

/// Outcome of consuming a stream: the accumulated chunk (None when
/// the stream produced zero items) + the count of chunks consumed.
pub struct Consumed<Chunk> {
    pub aggregate: Option<Chunk>,
    pub chunk_count: usize,
}

/// Drain `stream`, send each chunk as one
/// [`InstanceEmission::Chunk`] through `emissions_tx`, manage
/// per-agent pipes, coalesce-write to the log, emit
/// [`InstanceEmission::LogStreamReady`] once, accumulate. On stream
/// end (success or first error), tears down every active pipe and
/// waits for the writer task to flush its final batch.
///
/// `registry` is supplied by the caller so the endpoint-level eager
/// admission probe and the per-chunk `ensure_pipe` calls share one
/// registry instance. When a per-chunk `ensure_pipe` reports
/// [`BindStatus::Live`], the loop exits the process with
/// `SLOT_TAKEN_EXIT_CODE` so the wrapper CLI can recursively retry.
pub async fn run_chunk_loop<S, Chunk, E, F>(
    mut stream: S,
    notifier: Notifier,
    pipes_root: PathBuf,
    caller_agent_instance_hierarchy: Option<String>,
    log_writer: LogWriter<Chunk>,
    emissions_tx: EmissionsTx,
    push: F,
    registry: PipeRegistry,
) -> Result<Consumed<Chunk>, String>
where
    S: Stream<Item = Result<Chunk, E>> + Unpin,
    Chunk: AgentCompletionIds + Serialize + Clone + Send + Sync + 'static,
    E: std::fmt::Display,
    F: Fn(&mut Chunk, &Chunk) + Clone + Send + 'static,
{
    let mut aggregate: Option<Chunk> = None;
    let mut chunk_count: usize = 0;

    let (tx, rx) = mpsc::unbounded_channel::<Chunk>();
    let (notif_tx, notif_rx) =
        mpsc::unbounded_channel::<(String, String, RichContent)>();
    // One-shot carrying the primary log id from `writer_loop` back
    // here. `writer_loop` populates it once `log_writer.write(...)`
    // exposes a `primary_id`; this loop picks it up before its very
    // first `send_chunk` and emits `LogStreamReady` itself.
    //
    // The single-producer-to-`emissions_tx` invariant matters: with
    // only this loop ever touching `emissions_tx`, "LogStreamReady
    // is the first emission consumers see" is enforced structurally
    // by sequential execution rather than synchronised between two
    // parallel producers. `writer_loop` owns logs and nothing else.
    let (log_ready_id_tx, log_ready_id_rx) =
        tokio::sync::oneshot::channel::<String>();
    let mut log_ready_id_rx = Some(log_ready_id_rx);
    let writer_push = push.clone();
    let writer_registry = registry.clone();
    let writer_task = tokio::spawn(async move {
        writer_loop(
            rx,
            notif_rx,
            log_writer,
            writer_push,
            writer_registry,
            log_ready_id_tx,
        )
        .await
    });

    let mut stream_err: Option<String> = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                // 1. Hand the chunk to the writer task FIRST so it
                //    can flush a log entry and expose a primary id.
                //    Without this hand-off the oneshot below is
                //    never satisfied.
                let _ = tx.send(chunk.clone());

                // 2. On the very first iteration, pick up the
                //    primary id from the writer and emit
                //    `LogStreamReady` ourselves. Subsequent
                //    iterations skip this (the rx is taken). A
                //    writer error before the first write drops
                //    `log_ready_id_tx`, surfacing as `Err(Canceled)`
                //    here — we skip the LogStreamReady emit; the
                //    next `tx.send(chunk.clone())` will fail too
                //    because the writer's rx is gone, propagating
                //    the underlying failure through `writer_task`.
                if let Some(rx) = log_ready_id_rx.take() {
                    if let Ok(id) = rx.await {
                        send_log_stream_ready(&emissions_tx, &id).await;
                    }
                }

                // 3. Emit the chunk. By construction LogStreamReady
                //    is now ahead of every Chunk on `emissions_tx`.
                send_chunk(&emissions_tx, &chunk).await;

                // 4. Ensure a pipe is bound for every agent id this
                //    chunk references.
                for raw in chunk.agent_completion_ids() {
                    let lineage_id = match &caller_agent_instance_hierarchy {
                        Some(c) => format!("{c}/{raw}"),
                        None => raw.to_string(),
                    };
                    match registry
                        .ensure_pipe(
                            &lineage_id,
                            raw,
                            &pipes_root,
                            notifier.clone(),
                            notif_tx.clone(),
                        )
                        .await
                    {
                        Ok(()) => {}
                        Err(BindStatus::Live) => {
                            std::process::exit(crate::instance::api::SLOT_TAKEN_EXIT_CODE);
                        }
                        Err(BindStatus::Io) => {
                            // Degraded path — warning already eprintln'd.
                        }
                    }
                    registry
                        .ensure_outbound_pipe(&lineage_id, &pipes_root)
                        .await;
                }

                // 5. Accumulate main-side.
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

    drop(tx);
    registry.shutdown_inbound();
    drop(notif_tx);

    let writer_outcome = writer_task
        .await
        .map_err(|e| format!("log writer task panicked: {e}"))?;
    if let Err(e) = writer_outcome {
        registry.shutdown_outbound();
        return Err(format!("log writer failed: {e}"));
    }
    registry.shutdown_outbound();

    if let Some(e) = stream_err {
        return Err(e);
    }
    Ok(Consumed {
        aggregate,
        chunk_count,
    })
}

async fn writer_loop<Chunk, F>(
    mut rx: mpsc::UnboundedReceiver<Chunk>,
    mut notif_rx: mpsc::UnboundedReceiver<(String, String, RichContent)>,
    mut log_writer: LogWriter<Chunk>,
    push: F,
    registry: PipeRegistry,
    log_ready_id_tx: tokio::sync::oneshot::Sender<String>,
) -> Result<(), crate::filesystem::Error>
where
    F: Fn(&mut Chunk, &Chunk),
    Chunk: AgentCompletionIds + Clone,
{
    let mut agg: Option<Chunk> = None;
    let mut pending: Vec<PendingNotification> = Vec::new();
    // Held until we publish the first `primary_id` back to
    // `run_chunk_loop`; then `take()`d so subsequent iterations don't
    // try to re-send. Dropping it (writer error before any write)
    // wakes the matching await with `Err(Canceled)`, which
    // `run_chunk_loop` treats as "skip LogStreamReady".
    let mut log_ready_id_tx = Some(log_ready_id_tx);
    let mut chunk_channel_open = true;
    let mut notif_channel_open = true;

    while chunk_channel_open || notif_channel_open {
        tokio::select! {
            biased;
            chunk = rx.recv(), if chunk_channel_open => {
                match chunk {
                    Some(first) => {
                        match &mut agg {
                            Some(a) => push(a, &first),
                            None => agg = Some(first),
                        }
                        while let Ok(next) = rx.try_recv() {
                            if let Some(a) = &mut agg {
                                push(a, &next);
                            }
                        }
                        while let Ok((aid, response_id, content)) = notif_rx.try_recv() {
                            let p = log_writer
                                .write_notification(&aid, &response_id, &content)
                                .await?;
                            pending.push(p);
                        }
                        if let Some(a) = &agg {
                            let inserted = log_writer.write(a, &mut pending).await?;
                            broadcast_rows(&registry, &inserted);
                        }
                        if let Some(tx) = log_ready_id_tx.take() {
                            if let Some(id) = log_writer.primary_id() {
                                let _ = tx.send(id.to_string());
                            } else {
                                // No id yet — put the sender back
                                // so a later iteration can publish
                                // it once the writer settles.
                                log_ready_id_tx = Some(tx);
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

    let inserted = log_writer.finalize(&mut pending).await?;
    broadcast_rows(&registry, &inserted);
    registry.broadcast_stream_end();
    Ok(())
}

fn broadcast_rows(registry: &PipeRegistry, inserted: &[(String, RequestMessageKind)]) {
    for (agent_instance_hierarchy, kind) in inserted {
        if let Some(tx) = registry.outbound_sender(agent_instance_hierarchy) {
            let _ = tx.send(SubscribeEvent::Row {
                message_kind: *kind,
            });
        }
    }
}

async fn send_log_stream_ready(emissions_tx: &EmissionsTx, id: &str) {
    let _ = emissions_tx
        .send(Ok(InstanceEmission::LogStreamReady {
            log_stream_ready: id.to_string(),
        }))
        .await;
}

async fn send_chunk<C: Serialize>(emissions_tx: &EmissionsTx, chunk: &C) {
    let value = match serde_json::to_value(chunk) {
        Ok(v) => v,
        Err(_) => return,
    };
    let _ = emissions_tx
        .send(Ok(InstanceEmission::Chunk(value)))
        .await;
}
