//! Shared child-process stdout+stderr reader.
//!
//! Spawns one reader task per pipe; both push line events into a
//! single `mpsc::unbounded_channel`. The receiver yields events in
//! strict cross-producer FIFO arrival order — no `select!` over a
//! borrowed `read_line` future (which isn't cancel-safe), no
//! round-robin merge.
//!
//! Each reader task is a bare line-reader: it strips the trailing
//! `\r?\n` and forwards the line as a `PipeEvent::Stdout` /
//! `Stderr`. The caller decides what to do with each event (JSON
//! parse, wrap in `cli::Error`, discard content, etc.).

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc;

/// One event from a child's stdout/stderr pipe.
pub enum PipeEvent {
    /// A complete line read from the child's stdout, with the
    /// trailing `\r?\n` stripped.
    Stdout(String),
    /// A complete line read from the child's stderr, with the
    /// trailing `\r?\n` stripped.
    Stderr(String),
    /// Stdout reached EOF — its reader task has exited.
    StdoutEof,
    /// Stderr reached EOF — its reader task has exited.
    StderrEof,
    /// Stdout read errored. The reader task has exited.
    StdoutErr(std::io::Error),
    /// Stderr read errored. The reader task has exited.
    StderrErr(std::io::Error),
}

/// Spawn two reader tasks (one per pipe) and return an mpsc
/// receiver yielding [`PipeEvent`]s in arrival order. The channel
/// closes once both reader tasks have exited (EOF or read error).
///
/// Caller-side: `while let Some(event) = rx.recv().await { match … }`.
pub fn spawn_pipe_reader<O, E>(
    stdout: O,
    stderr: E,
) -> mpsc::UnboundedReceiver<PipeEvent>
where
    O: AsyncRead + Unpin + Send + 'static,
    E: AsyncRead + Unpin + Send + 'static,
{
    let (tx, rx) = mpsc::unbounded_channel();
    let stdout_tx = tx.clone();
    tokio::spawn(async move {
        read_lines(
            stdout,
            stdout_tx,
            PipeEvent::Stdout,
            PipeEvent::StdoutEof,
            PipeEvent::StdoutErr,
        )
        .await;
    });
    tokio::spawn(async move {
        read_lines(
            stderr,
            tx,
            PipeEvent::Stderr,
            PipeEvent::StderrEof,
            PipeEvent::StderrErr,
        )
        .await;
    });
    rx
}

/// Read lines off `reader` and send each as a [`PipeEvent`] until
/// EOF or read error. `line_event` builds a per-line event from the
/// trimmed line text; `eof_event` and `err_event_ctor` mint the
/// terminal event before the task exits.
async fn read_lines<R, F, G>(
    reader: R,
    tx: mpsc::UnboundedSender<PipeEvent>,
    line_event: F,
    eof_event: PipeEvent,
    err_event_ctor: G,
) where
    R: AsyncRead + Unpin,
    F: Fn(String) -> PipeEvent,
    G: FnOnce(std::io::Error) -> PipeEvent,
{
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                let _ = tx.send(eof_event);
                return;
            }
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\r', '\n']).to_string();
                if tx.send(line_event(trimmed)).is_err() {
                    return;
                }
            }
            Err(e) => {
                let _ = tx.send(err_event_ctor(e));
                return;
            }
        }
    }
}
