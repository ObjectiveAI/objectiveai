//! Shared chunk-consumption loop every endpoint reuses.
//!
//! Drains the WS chunk stream, prints each chunk as one NDJSON line
//! on stdout (matches the existing `objectiveai-cli`'s output
//! convention), ensures a per-agent pipe exists for every
//! `agent_completion_id` the chunk references, accumulates chunks
//! into a final aggregate via the caller-supplied `push` closure,
//! and on stream end fires every pipe canceller.

use std::path::PathBuf;

use futures::{Stream, StreamExt};
use objectiveai_sdk::Notifier;
use objectiveai_sdk::agent::completions::response::streaming::AgentCompletionIds;
use objectiveai_sdk::cli::output::Handle;
use serde::Serialize;

use crate::pipes::PipeRegistry;

/// Outcome of consuming a stream: the accumulated chunk (None when
/// the stream produced zero items) + the count of chunks consumed.
pub struct Consumed<Chunk> {
    pub aggregate: Option<Chunk>,
    pub chunk_count: usize,
}

/// Drain `stream`, emit each chunk as NDJSON to `handle`'s stdout
/// destination, manage per-agent pipes, accumulate. On stream end
/// (success or first error), tears down every active pipe.
pub async fn run_chunk_loop<S, Chunk, E>(
    mut stream: S,
    notifier: Notifier,
    pipes_root: PathBuf,
    handle: &Handle,
    mut push: impl FnMut(&mut Chunk, &Chunk),
) -> Result<Consumed<Chunk>, E>
where
    S: Stream<Item = Result<Chunk, E>> + Unpin,
    Chunk: AgentCompletionIds + Serialize,
{
    let registry = PipeRegistry::new();
    let mut aggregate: Option<Chunk> = None;
    let mut chunk_count: usize = 0;

    let mut stream_err: Option<E> = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                // 1. Emit the chunk to stdout as one NDJSON line.
                emit_chunk(&chunk, handle).await;

                // 2. Ensure a pipe is bound for every agent id this
                //    chunk references. `ensure_pipe` is idempotent —
                //    repeat ids are no-ops.
                for agent_id in chunk.agent_completion_ids() {
                    registry
                        .ensure_pipe(agent_id, &pipes_root, notifier.clone(), handle)
                        .await;
                }

                // 3. Accumulate.
                match aggregate.as_mut() {
                    Some(acc) => push(acc, &chunk),
                    None => aggregate = Some(chunk),
                }
                chunk_count += 1;
            }
            Err(e) => {
                stream_err = Some(e);
                break;
            }
        }
    }

    // Tear down every pipe. Reader tasks wake from their
    // tokio::select! and unlink the FS entry on POSIX.
    registry.shutdown();

    if let Some(e) = stream_err {
        return Err(e);
    }
    Ok(Consumed {
        aggregate,
        chunk_count,
    })
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
    let out =
        objectiveai_sdk::cli::output::Output::<serde_json::Value>::Notification(
            objectiveai_sdk::cli::output::Notification {
                agent_id: None,
                value,
            },
        );
    out.emit(handle).await;
}
