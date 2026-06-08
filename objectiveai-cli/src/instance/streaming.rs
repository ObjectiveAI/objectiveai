//! Shared chunk-consumption loop every endpoint reuses.
//!
//! Drains the WS chunk stream, sends each chunk as an
//! [`InstanceEmission::Chunk`] through `emissions_tx`, writes each
//! chunk to a [`LogWriter`] on a separate coalescing task (so log
//! writes don't gate stream consumption), accumulates chunks into
//! a final aggregate via the caller-supplied `push` closure, and
//! emits a one-shot [`InstanceEmission::LogStreamReady`] with the
//! root log id as soon as the first write completes.

use std::path::PathBuf;

use futures::{Stream, StreamExt};
use objectiveai_sdk::agent::completions::response::streaming::AgentCompletionIds;
use serde::Serialize;
use tokio::sync::mpsc;

use crate::error::Error;
use crate::db::logs::LogWriter;
use crate::instance::InstanceEmission;
use crate::instance::agent_hierarchies::ChunkAgentHierarchies;
use crate::agent_registry::AgentInstanceRegistry;

pub type EmissionsTx = mpsc::Sender<Result<InstanceEmission, Error>>;

/// Outcome of consuming a stream: the accumulated chunk (None when
/// the stream produced zero items) + the count of chunks consumed.
pub struct Consumed<Chunk> {
    pub aggregate: Option<Chunk>,
    pub chunk_count: usize,
}

/// Drain `stream`, send each chunk as one
/// [`InstanceEmission::Chunk`] through `emissions_tx`, coalesce-write
/// to the log, emit [`InstanceEmission::LogStreamReady`] once,
/// accumulate.
pub async fn run_chunk_loop<S, Chunk, E, F>(
    mut stream: S,
    log_writer: LogWriter<Chunk>,
    emissions_tx: EmissionsTx,
    push: F,
    agents_dir: PathBuf,
) -> Result<Consumed<Chunk>, String>
where
    S: Stream<Item = Result<Chunk, E>> + Unpin,
    Chunk: crate::db::logs::WriterChunk
        + AgentCompletionIds
        + ChunkAgentHierarchies
        + Serialize
        + Clone
        + Send
        + Sync
        + 'static,
    E: std::fmt::Display,
    F: Fn(&mut Chunk, &Chunk) + Clone + Send + 'static,
{
    // Process-owned exclusive claims on every agent_instance_hierarchy
    // observed in the stream. Dropped when the loop returns, releasing
    // every still-held claim. `new` only fails when the root directory
    // can't be created (permission denied, disk full, etc.) — that's a
    // genuine environmental failure, not best-effort territory.
    let mut seen_agents = AgentInstanceRegistry::new(agents_dir)
        .map_err(|e| format!("failed to open agent claim registry: {e}"))?;
    let mut aggregate: Option<Chunk> = None;
    let mut chunk_count: usize = 0;

    let (tx, rx) = mpsc::unbounded_channel::<Chunk>();
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
    let writer_task = tokio::spawn(async move {
        writer_loop(rx, log_writer, writer_push, log_ready_id_tx).await
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
                        // 0. Best-effort claim of every
                        //    agent_instance_hierarchy referenced in this
                        //    chunk. Per-chunk dispatch — the registry's
                        //    internal HashMap dedupes, so each new
                        //    hierarchy becomes a live OS-managed lock
                        //    file the moment it first appears on the
                        //    wire. Failures (already claimed elsewhere,
                        //    illegal chars, etc.) are silently dropped.
                        for hier in chunk.agent_instance_hierarchies() {
                            seen_agents.observe(hier);
                        }

                        // 1. Hand a clone to the writer task so it can flush
                        //    a log entry and expose `primary_id`.
                        let _ = tx.send(chunk.clone());

                        // 2. Emit this chunk directly, or buffer for later.
                        if log_ready_emitted {
                            send_chunk(&emissions_tx, &chunk).await;
                        } else {
                            buffered.push(chunk.clone());
                        }

                        // 3. Accumulate main-side.
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

    // Stream EOF — close the writer's input channel so its
    // `finalize` runs. `finalize` processes the last chunk still
    // buffered behind `log_writer`'s one-behind write semantics, so
    // for single-chunk completions this is the only point at which
    // `primary_id` becomes available.
    drop(tx);

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

async fn writer_loop<Chunk, F>(
    mut rx: mpsc::UnboundedReceiver<Chunk>,
    mut log_writer: LogWriter<Chunk>,
    push: F,
    log_ready_id_tx: tokio::sync::oneshot::Sender<String>,
) -> Result<(), crate::error::Error>
where
    F: Fn(&mut Chunk, &Chunk),
    Chunk: crate::db::logs::WriterChunk
        + AgentCompletionIds
        + Serialize
        + Clone
        + Send
        + Sync
        + 'static,
{
    let mut agg: Option<Chunk> = None;
    // Held until `log_writer.primary_id()` returns Some — could fire
    // mid-loop (multi-chunk completions; `write` flushes the
    // previous chunk on its second-and-later call) or only from
    // `finalize` below (single-chunk completions). Drop without
    // `take` = chunk loop sees `Err(Canceled)` and emits any buffered
    // chunks without LogStreamReady.
    let mut log_ready_id_tx = Some(log_ready_id_tx);

    while let Some(first) = rx.recv().await {
        match &mut agg {
            Some(a) => push(a, &first),
            None => agg = Some(first),
        }
        while let Ok(next) = rx.try_recv() {
            if let Some(a) = &mut agg {
                push(a, &next);
            }
        }
        if let Some(a) = &agg {
            log_writer.write(a).await?;
        }
        if let Some(tx) = log_ready_id_tx.take() {
            if let Some(id) = log_writer.primary_id() {
                let _ = tx.send(id.to_string());
            } else {
                log_ready_id_tx = Some(tx);
            }
        }
    }

    log_writer.finalize().await?;

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

    Ok(())
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
