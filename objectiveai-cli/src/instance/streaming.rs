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
    // One-shot carrying the primary log id back here from
    // `writer_loop` once `log_writer.primary_id()` has been populated.
    // `writer_loop` owns the sender and fires it the first time
    // primary_id becomes available (which may be mid-stream or only
    // during `finalize`, depending on the chunk shape). Until the
    // signal arrives we buffer chunks below so consumers see
    // `LogStreamReady, Chunk, Chunk, …` — never a Chunk before the
    // LogStreamReady.
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

    // Local buffer for chunks held back until the writer signals it
    // has a primary id. Once we receive the signal we emit
    // `LogStreamReady` + drain this buffer, and subsequent iterations
    // emit chunks directly. Bounded in practice by however many
    // chunks the stream produces before the writer flushes its first
    // log file — typically 1-3.
    let mut buffered: Vec<Chunk> = Vec::new();
    let mut log_ready_emitted = false;

    let mut stream_err: Option<String> = None;
    loop {
        tokio::select! {
            biased;

            // The writer's primary_id landed. Emit `LogStreamReady`,
            // drain everything we've been holding back, and from here
            // on each chunk emits directly. This branch runs at most
            // once per stream — disabled the moment `log_ready_emitted`
            // flips. By keeping it in the same `select!` as the chunk
            // stream we react the instant the oneshot fires instead
            // of waiting for the next chunk to come around and poll
            // `try_recv` — important when the api goes quiet between
            // the last burst of chunks and the WS close, which is
            // exactly the window the watchdog used to fire on.
            ready_result = async {
                log_ready_id_rx.as_mut().unwrap().await
            }, if !log_ready_emitted && log_ready_id_rx.is_some() => {
                log_ready_id_rx = None;
                log_ready_emitted = true;
                if let Ok(id) = ready_result {
                    send_log_stream_ready(&emissions_tx, &id).await;
                }
                for buf in buffered.drain(..) {
                    send_chunk(&emissions_tx, &buf).await;
                }
            }

            item = stream.next() => {
                match item {
                    Some(Ok(chunk)) => {
                        // 1. Hand a clone to the writer task so it can flush
                        //    a log entry and expose `primary_id`.
                        let _ = tx.send(chunk.clone());

                        // 2. Emit this chunk directly, or buffer for later.
                        if log_ready_emitted {
                            send_chunk(&emissions_tx, &chunk).await;
                        } else {
                            buffered.push(chunk.clone());
                        }

                        // 3. Ensure a pipe is bound for every agent id this
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

                        // 4. Accumulate main-side.
                        match aggregate.as_mut() {
                            Some(acc) => push(acc, &chunk),
                            None => aggregate = Some(chunk),
                        }
                        chunk_count += 1;
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

    // Stream EOF — close the writer's input channels so its
    // `finalize` runs. `finalize` processes the last chunk still
    // buffered behind `log_writer`'s one-behind write semantics, so
    // for single-chunk completions this is the only point at which
    // `primary_id` becomes available.
    drop(tx);
    registry.shutdown_inbound();
    drop(notif_tx);

    // If we never saw the log-ready signal mid-stream, await it now
    // — the writer is racing toward `finalize` which fires it once
    // primary_id lands. On writer error before any primary_id, the
    // sender drops and the await resolves with `Err(Canceled)`; we
    // still emit the buffered chunks for visibility into what
    // arrived before the failure. The writer error itself surfaces
    // below via `writer_task.await`.
    if !log_ready_emitted {
        if let Some(rx) = log_ready_id_rx.take() {
            if let Ok(id) = rx.await {
                send_log_stream_ready(&emissions_tx, &id).await;
            }
            for buf in buffered.drain(..) {
                send_chunk(&emissions_tx, &buf).await;
            }
        }
    }

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
    // Held until `log_writer.primary_id()` returns Some — could fire
    // mid-loop (multi-chunk completions; `write` flushes the
    // previous chunk on its second-and-later call) or only from
    // `finalize` below (single-chunk completions, where the only
    // chunk sits in `log_writer`'s one-behind buffer until shutdown).
    // Drop without `take` = chunk loop sees `Err(Canceled)` and
    // emits any buffered chunks without LogStreamReady.
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

    // Last-chance fire — the chunk that was sitting in
    // `log_writer`'s one-behind buffer just got flushed by
    // `finalize`, so this is the latest possible point primary_id
    // can land. If we still don't have one (e.g. zero-chunk
    // completion) the sender drops here and the receiver wakes with
    // `Err(Canceled)` in `run_chunk_loop`.
    if let Some(tx) = log_ready_id_tx.take() {
        if let Some(id) = log_writer.primary_id() {
            let _ = tx.send(id.to_string());
        }
    }

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
