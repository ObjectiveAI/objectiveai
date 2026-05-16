//! Streaming helper that decouples log-writing from stream consumption.
//!
//! Every CLI `create` command (function executions, agent completions,
//! recursive inventions, laboratory executions) streams chunks from the API
//! and needs to persist an aggregated view to disk as it grows. Writing
//! synchronously on every chunk is prohibitively slow for large aggregates:
//! `LogWriter::write` re-serializes the full aggregate to JSON (to diff it
//! against its buffer), so wall-clock is dominated by serialization that
//! gets thrown away because the content hasn't changed.
//!
//! [`consume_with_coalesced_writes`] fixes this by running the log writer
//! on a separate tokio task. Chunks reach the writer via an unbounded
//! channel. Each iteration the writer task blocks for at least one chunk,
//! then drains anything else already queued, **merges the batch** into a
//! single aggregate via the caller-supplied `push` delegate, and issues
//! one `LogWriter::write` for the whole batch.
//!
//! When the producer is faster than the writer, multiple chunks coalesce
//! into a single write. When the writer is faster, this degenerates to
//! one-in-one-out. The final on-disk state is identical either way —
//! `push` is a monoid, so the accumulated state only depends on the
//! ordered chunk sequence, not on where batch boundaries fall.

use futures::{Stream, StreamExt};

/// Consume a chunk stream while log-writing runs on a separate coalescing
/// task.
///
/// Returns the fully-merged main-side aggregate, or an error if the stream
/// or the writer produced one. The main-side aggregate and the writer-side
/// aggregate stay in sync because both receive the same ordered chunk
/// sequence — the caller only ever sees the main-side one.
///
/// The `push` closure merges a new chunk into the running aggregate. Both
/// the main-side and writer-side aggregates use it (`Clone` is required
/// so the writer task can own its own copy).
pub(crate) async fn consume_with_coalesced_writes<C, F>(
    stream: impl Stream<Item = Result<C, crate::error::Error>>,
    log_writer: objectiveai_sdk::filesystem::logs::LogWriter<C>,
    push: F,
    handle: objectiveai_sdk::cli::output::Handle,
) -> Result<C, crate::error::Error>
where
    C: Clone + Send + Sync + 'static,
    F: Fn(&mut C, &C) + Clone + Send + 'static,
{
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<C>();
    let writer_push = push.clone();
    let writer_handle = tokio::spawn(async move {
        writer_loop(rx, log_writer, writer_push, handle).await
    });

    let mut main_agg: Option<C> = None;
    let mut stream_err: Option<crate::error::Error> = None;
    futures::pin_mut!(stream);
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                match &mut main_agg {
                    Some(a) => push(a, &chunk),
                    None => main_agg = Some(chunk.clone()),
                }
                // Best-effort: writer may have errored out and dropped rx.
                let _ = tx.send(chunk);
            }
            Err(e) => {
                stream_err = Some(e);
                break;
            }
        }
    }
    // Signal the writer task that no more chunks are coming.
    drop(tx);

    // Wait for the writer task to finish its final batch. `JoinError` is
    // only produced on panic/cancellation; a writer error (disk full, etc.)
    // comes back as `Err(Error)` from the task's own return value.
    let writer_result = writer_handle
        .await
        .map_err(|_| crate::error::Error::WriterPanic)?;
    writer_result?;

    if let Some(e) = stream_err {
        return Err(e);
    }
    main_agg.ok_or(crate::error::Error::EmptyStream)
}

/// The coalescing loop: block on one chunk, drain any that piled up during
/// the previous write, merge, write once. Repeat until the channel closes.
async fn writer_loop<C, F>(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<C>,
    mut log_writer: objectiveai_sdk::filesystem::logs::LogWriter<C>,
    push: F,
    handle: objectiveai_sdk::cli::output::Handle,
) -> Result<(), crate::error::Error>
where
    F: Fn(&mut C, &C),
{
    let mut agg: Option<C> = None;
    let mut logged_id = false;
    while let Some(first) = rx.recv().await {
        match &mut agg {
            Some(a) => push(a, &first),
            None => agg = Some(first),
        }
        // Drain anything that arrived while we were blocked on `recv` or
        // while the previous `write` was running. A non-empty drain is what
        // makes this coalescing rather than plain batching.
        while let Ok(next) = rx.try_recv() {
            if let Some(a) = &mut agg {
                push(a, &next);
            }
        }
        if let Some(a) = &agg {
            log_writer.write(a).await?;
        }
        if !logged_id {
            if let Some(id) = log_writer.primary_id() {
                crate::log_line::emit_log_stream_ready(id, &handle).await;
                logged_id = true;
            }
        }
    }
    Ok(())
}
