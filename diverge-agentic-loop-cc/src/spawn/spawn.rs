//! Launching the subprocess, and the reader task beside it.

use std::io;
use std::process::Stdio;

use diverge_provider_sdk::agentic_loop_container;
use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::{
    AgenticLoopChunk, UserChunk,
};
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
/// What Claude Code writes reaches `sender` CONVERTED: each line
/// parses strictly per the [`response`] module's contract and
/// becomes the chunks it means — most records mean none — with a
/// parse failure travelling as the `Err` it is, for the consumer to
/// judge. The receiver's death stops nothing.
pub async fn spawn(
    continuation: Option<continuation::Continuation>,
    prompt: String,
    sender: mpsc::UnboundedSender<
        Result<AgenticLoopChunk, serde_json::Error>,
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
/// Per line: parse strictly, then two taps before conversion. A
/// `control_response` record goes, cloned, through the reply
/// sender — the receiver half sits behind [`replies::REPLIES`], so
/// only that lock's holder reads it. A replay echo whose uuid is in
/// [`pending::PENDING`] is a delivery: the fate resolves
/// `delivered` — strict fates' third arm — and the `user` chunk
/// carrying the prompt is pushed at exactly this position, which is
/// the protocol's: Claude Code yields the tool-response records
/// BEFORE the replay, so the mark lands behind the answers it
/// followed. A replay with no pending entry — the initial prompt's
/// echo, resumed history, a message dequeued mid-delivery — taps
/// nothing. Then the record converts, and every chunk forwards
/// through the caller's sender; a line that failed to parse
/// forwards as its error.
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
        Result<AgenticLoopChunk, serde_json::Error>,
    >,
) {
    let mut lines = BufReader::new(child_stdout).lines();
    // One buffer for the whole stream: each record's chunks land
    // here, drain to the sender, and the allocation stays.
    let mut chunks: Vec<AgenticLoopChunk> = Vec::new();
    while let Ok(Some(line)) = lines.next_line().await {
        let record =
            match serde_json::from_str::<response::StdoutMessage>(&line)
            {
                Ok(record) => record,
                Err(error) => {
                    let _ = sender.send(Err(error));
                    continue;
                }
            };
        match &record {
            // The cancel-reply tap.
            response::StdoutMessage::ControlResponse(reply) => {
                let _ = reply_sender.send(reply.clone());
            }
            // The delivery tap.
            response::StdoutMessage::User(user)
                if user.is_replay == Some(true) =>
            {
                if let Some(uuid) = &user.uuid {
                    if let Some((_, fate)) =
                        pending::PENDING.remove(uuid)
                    {
                        let _ = fate.send(
                            agentic_loop_container::enqueue::Response::Delivered {
                                r#type: Default::default(),
                            },
                        );
                        chunks.push(AgenticLoopChunk::User(
                            UserChunk {
                                r#type: Default::default(),
                                prompt: user.message.plain_text(),
                                meta: None,
                            },
                        ));
                    }
                }
            }
            _ => {}
        }
        record.into_chunks(&mut chunks);
        for chunk in chunks.drain(..) {
            let _ = sender.send(Ok(chunk));
        }
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
