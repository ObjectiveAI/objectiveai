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
    mut on_chunk_response_ids: Option<
        Box<dyn FnMut(&std::collections::HashSet<String>) + Send>,
    >,
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
    let writer_push = push.clone();
    let writer_emissions_tx = emissions_tx.clone();
    let writer_registry = registry.clone();
    let writer_task = tokio::spawn(async move {
        writer_loop(
            rx,
            notif_rx,
            log_writer,
            writer_push,
            writer_emissions_tx,
            writer_registry,
        )
        .await
    });

    let mut stream_err: Option<String> = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                // 1. Send the chunk as a typed emission.
                send_chunk(&emissions_tx, &chunk).await;

                // 2. Ensure a pipe is bound for every agent id this
                //    chunk references.
                let mut response_ids_this_chunk: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                for raw in chunk.agent_completion_ids() {
                    response_ids_this_chunk.insert(raw.to_string());
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

                if !response_ids_this_chunk.is_empty() {
                    if let Some(cb) = on_chunk_response_ids.as_mut() {
                        cb(&response_ids_this_chunk);
                    }
                }

                // 3. Hand a clone to the writer task.
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
    emissions_tx: EmissionsTx,
    registry: PipeRegistry,
) -> Result<(), crate::filesystem::Error>
where
    F: Fn(&mut Chunk, &Chunk),
    Chunk: AgentCompletionIds + Clone,
{
    let mut agg: Option<Chunk> = None;
    let mut pending: Vec<PendingNotification> = Vec::new();
    let mut logged_id = false;
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
                        if !logged_id {
                            if let Some(id) = log_writer.primary_id() {
                                send_log_stream_ready(&emissions_tx, id).await;
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
