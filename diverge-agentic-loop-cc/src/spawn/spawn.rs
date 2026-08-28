//! Launching the subprocess, and the reader task beside it.

use std::io;
use std::process::Stdio;

use diverge_provider_sdk::agentic_loop_container;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::continuation;
use crate::response;

use super::pending;
use super::replies;
use super::stdin;
use super::writer;

/// Start the run: write the continuation's files if resuming, launch
/// Claude Code, hand it the prompt, and leave the reader task
/// running.
///
/// The caller ensures there is one run per container lifetime — the
/// same door guard the root handler owns — so this never contends
/// with an earlier subprocess.
///
/// Every line Claude Code writes reaches `sender` as its parse
/// result, strict per the [`response`] module's contract; the
/// receiver's death stops nothing.
pub async fn spawn(
    continuation: Option<continuation::Continuation>,
    prompt: String,
    sender: mpsc::UnboundedSender<
        Result<response::StdoutMessage, serde_json::Error>,
    >,
) -> io::Result<()> {
    // Resuming is the files existing before Claude Code starts.
    let session_id = match &continuation {
        Some(continuation) => {
            continuation.write().await?;
            Some(continuation.session_id.clone())
        }
        None => None,
    };

    let mut command = process::Command::new("claude");
    command
        .arg("-p")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--input-format")
        .arg("stream-json")
        .arg("--replay-user-messages")
        .arg("--dangerously-skip-permissions")
        .arg("--mcp-config")
        .arg(
            r#"{"mcpServers":{"diverge":{"type":"http","url":"http://localhost:8081/mcp"}}}"#,
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    if let Some(session_id) = &session_id {
        command.arg("--resume").arg(session_id);
    }
    let mut child = command.spawn()?;
    let mut child_stdin = child.stdin.take().expect("stdin piped above");
    let child_stdout = child.stdout.take().expect("stdout piped above");

    // The run's prompt: stream-json input mode's way of delivering
    // the print prompt is the same line an enqueue writes. Its uuid
    // is NOT queued — the run's prompt is not steerable, so no
    // dequeue may cancel it.
    stdin::write_lines(
        &mut child_stdin,
        &stdin::user_message_line(prompt, Uuid::new_v4().to_string()),
    )
    .await?;

    let (reply_sender, reply_receiver) = mpsc::unbounded_channel();
    // Replies first, writer second: enqueue and dequeue gate on the
    // writer, so a writer seen must imply the replies are already in
    // place.
    *replies::REPLIES.lock().await = Some(reply_receiver);
    *writer::WRITER.lock().await = Some(child_stdin);
    tokio::spawn(read(child_stdout, child, reply_sender, sender));
    Ok(())
}

/// The reader task: the one reader of stdout, and the child's reaper.
///
/// Per line: parse strictly; a `control_response` record additionally
/// goes, cloned, through the reply sender — the receiver half sits
/// behind [`replies::REPLIES`], so only that lock's holder reads it —
/// and every parse result forwards through the caller's sender
/// regardless.
///
/// When the stream ends — EOF, or an IO error ends the same way —
/// the ORDER is load-bearing: the reply sender drops FIRST, so a
/// dequeue holding the locks mid-wait wakes on the closed channel
/// and resolves; then the writer and replies locks are taken ONE AT
/// A TIME — never nested, so no cycle with the dequeue's joined
/// hold — and cleared (idempotent with that dequeue's own clearing);
/// and only AFTER the writer is `None` are the pending fates missed —
/// no enqueue can register a fate once the writer is gone, so
/// nothing slips in behind the drain. Then the child is reaped, and
/// dropping the caller's sender is the end-of-stream.
async fn read(
    child_stdout: process::ChildStdout,
    mut child: process::Child,
    reply_sender: mpsc::UnboundedSender<
        response::control::ControlResponse,
    >,
    sender: mpsc::UnboundedSender<
        Result<response::StdoutMessage, serde_json::Error>,
    >,
) {
    let mut lines = BufReader::new(child_stdout).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let parsed =
            serde_json::from_str::<response::StdoutMessage>(&line);
        if let Ok(response::StdoutMessage::ControlResponse(reply)) =
            &parsed
        {
            let _ = reply_sender.send(reply.clone());
        }
        let _ = sender.send(parsed);
    }
    drop(reply_sender);
    *writer::WRITER.lock().await = None;
    *replies::REPLIES.lock().await = None;
    // The run is over: every fate still undecided is now decided.
    let uuids: Vec<String> = pending::PENDING
        .iter()
        .map(|entry| entry.key().clone())
        .collect();
    for uuid in uuids {
        if let Some((_, fate)) = pending::PENDING.remove(&uuid) {
            let _ = fate.send(
                agentic_loop_container::enqueue::Response::Missed {
                    r#type: Default::default(),
                },
            );
        }
    }
    let _ = child.wait().await;
}
