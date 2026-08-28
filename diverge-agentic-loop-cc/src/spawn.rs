//! The Claude Code subprocess, and the two globals around it.
//!
//! One run is one subprocess, and one global mutex holds everything a
//! writer needs: [`SESSION`] is stdin, the queued uuids, and the
//! RECEIVER for cancel replies, together under one lock. The reader
//! task permanently holds the matching sender — so the lock's hold IS
//! the protocol's atomicity. An enqueue locks, writes, answers well.
//! A dequeue locks and KEEPS the lock across its cancel writes and
//! its reply reads: nothing can enqueue while it waits, so merely
//! receiving the replies means the withdrawal resolved.
//!
//! Beside the lock, [`DELIVERED`]: uuids known to have entered the
//! conversation, which lets a dequeue clean the queued vector before
//! writing cancels for messages that are already gone. The reader
//! will populate it from the replay echoes later; today it only
//! exists to be consulted.
//!
//! Deadlock audit: the reader never touches [`SESSION`] while
//! reading — only once, at end of stream, AFTER dropping the reply
//! sender — so a dequeue mid-wait always wakes (on `None` if the run
//! ends under it), and stdout always drains while an enqueue blocks
//! on a full stdin pipe.

use std::io;
use std::process::Stdio;
use std::sync::LazyLock;

use dashmap::DashSet;
use diverge_provider_sdk::agentic_loop_container;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process;
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

use crate::continuation;
use crate::response;
use crate::stdin;

/// The running session, or `None` before the run starts and after it
/// ends. Set by [`spawn`], taken out by whoever finds the process
/// dead first — a failed write, a dequeue waking on a closed reply
/// channel, or the reader at end of stream.
static SESSION: Mutex<Option<Session>> = Mutex::const_new(None);

/// Uuids of messages known to have entered the conversation.
///
/// Not populated yet — the reader will mark replay echoes here when
/// the conversion work lands — but already consulted:
/// [`dequeue`] drops these from the queued vector before writing
/// cancels, so a queue whose every message already landed is
/// honestly empty.
static DELIVERED: LazyLock<DashSet<String>> = LazyLock::new(DashSet::new);

/// What the one lock protects: the writer and everything whose
/// consistency rides on write order.
struct Session {
    /// The subprocess's stdin: the only way in.
    stdin: process::ChildStdin,
    /// Uuids of enqueued messages, in write order. Never pruned on
    /// delivery — only a dequeue's pre-clean does that, via
    /// [`DELIVERED`] — so entries may name messages already landed;
    /// cancelling those is Claude Code's documented no-op.
    queued: Vec<String>,
    /// Where the reader's forwarded cancel replies arrive. The
    /// receiver lives INSIDE the lock so that reading it is a right
    /// only the lock holder has — which is what lets a dequeue treat
    /// the replies it reads as answers to the cancels it wrote.
    replies: mpsc::UnboundedReceiver<response::control::ControlResponse>,
}

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
    write_lines(
        &mut child_stdin,
        &user_message_line(prompt, Uuid::new_v4().to_string()),
    )
    .await?;

    let (reply_sender, replies) = mpsc::unbounded_channel();
    *SESSION.lock().await = Some(Session {
        stdin: child_stdin,
        queued: Vec::new(),
        replies,
    });
    tokio::spawn(read(child_stdout, child, reply_sender, sender));
    Ok(())
}

/// Queue a message for the running session.
///
/// The write landing is the good answer: Claude Code holds the queue
/// from here, and short of a dequeue withdrawing it the message will
/// enter the conversation. No run to write to — never started, or
/// already over — is the expected failure, and the message is
/// missed.
pub async fn enqueue(
    prompt: String,
) -> agentic_loop_container::enqueue::Response {
    let mut session = SESSION.lock().await;
    let Some(inner) = session.as_mut() else {
        return agentic_loop_container::enqueue::Response::Missed {
            r#type: Default::default(),
        };
    };
    let uuid = Uuid::new_v4().to_string();
    match write_lines(
        &mut inner.stdin,
        &user_message_line(prompt, uuid.clone()),
    )
    .await
    {
        Ok(()) => {
            inner.queued.push(uuid);
            agentic_loop_container::enqueue::Response::Delivered {
                r#type: Default::default(),
            }
        }
        // A broken stdin is the process dying: the session is over.
        Err(_) => {
            *session = None;
            agentic_loop_container::enqueue::Response::Missed {
                r#type: Default::default(),
            }
        }
    }
}

/// Withdraw everything still queued.
///
/// The lock is held from the first look to the answer — across the
/// cancel writes AND the reply reads — and that hold is the whole
/// correctness: no enqueue can interleave, so the replies read are
/// answers to the cancels written, and receiving them means the
/// withdrawal resolved. No timeout; the wait is as long as Claude
/// Code takes, and a run that ends under the wait closes the reply
/// channel, which resolves it too.
pub async fn dequeue() -> agentic_loop_container::dequeue::Response {
    let mut session = SESSION.lock().await;
    let Some(inner) = session.as_mut() else {
        return agentic_loop_container::dequeue::Response::Empty {
            r#type: Default::default(),
        };
    };

    // Pre-clean: a message known delivered is not in the queue, and
    // writing a cancel for it would be asking about the past.
    inner.queued.retain(|uuid| !DELIVERED.contains(uuid));
    if inner.queued.is_empty() {
        return agentic_loop_container::dequeue::Response::Empty {
            r#type: Default::default(),
        };
    }

    // One buffered write for all the withdrawals, each under a fresh
    // request id; the replies quote the ids back.
    let mut request_ids = std::collections::HashSet::new();
    let mut lines = String::new();
    for uuid in &inner.queued {
        let request_id = Uuid::new_v4().to_string();
        lines.push_str(
            &serde_json::to_string(&stdin::ControlRequest {
                r#type: Default::default(),
                request_id: request_id.clone(),
                request: stdin::CancelAsyncMessage {
                    subtype: Default::default(),
                    message_uuid: uuid.clone(),
                },
            })
            .expect("a stdin line is plain structs and serializes"),
        );
        lines.push('\n');
        request_ids.insert(request_id);
    }
    if write_lines(&mut inner.stdin, &lines).await.is_err() {
        // A broken stdin is the process dying: the session is over,
        // and a dead queue holds nothing.
        *session = None;
        return agentic_loop_container::dequeue::Response::Empty {
            r#type: Default::default(),
        };
    }

    // Every cancel written gets its reply read, matched by request
    // id. A reply quoting an unknown id is stale — a prior dequeue
    // whose HTTP caller vanished mid-wait left it unread — and is
    // skipped, not counted.
    while !request_ids.is_empty() {
        match inner.replies.recv().await {
            Some(reply) => {
                let (response::control::ControlResponseInner::Success {
                    request_id,
                    ..
                }
                | response::control::ControlResponseInner::Error {
                    request_id,
                    ..
                }) = &reply.response;
                request_ids.remove(request_id);
            }
            // The reader dropped the sender: the run is over, and a
            // dead queue holds nothing.
            None => {
                *session = None;
                return agentic_loop_container::dequeue::Response::Empty {
                    r#type: Default::default(),
                };
            }
        }
    }

    inner.queued.clear();
    agentic_loop_container::dequeue::Response::Dequeued {
        r#type: Default::default(),
    }
}

/// The reader task: the one reader of stdout, and the child's reaper.
///
/// Per line: parse strictly; a `control_response` record additionally
/// goes, cloned, through the reply sender — the receiver half sits
/// inside [`SESSION`], so only a lock holder reads it — and every
/// parse result forwards through the caller's sender regardless.
///
/// When the stream ends — EOF, or an IO error ends the same way —
/// the ORDER is load-bearing: the reply sender drops FIRST, so a
/// dequeue holding the session lock mid-wait wakes on the closed
/// channel and resolves; only then is the lock taken to clear the
/// session (idempotent with that dequeue's own clearing). Then the
/// child is reaped, and dropping the caller's sender is the
/// end-of-stream.
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
    *SESSION.lock().await = None;
    let _ = child.wait().await;
}

/// One enqueued (or initial) prompt, as its NDJSON line.
fn user_message_line(prompt: String, uuid: String) -> String {
    let mut line = serde_json::to_string(&stdin::UserMessage {
        r#type: Default::default(),
        message: stdin::UserMessageBody {
            role: Default::default(),
            content: prompt,
        },
        uuid,
    })
    .expect("a stdin line is plain structs and serializes");
    line.push('\n');
    line
}

/// Write already-newline-terminated lines, then flush once.
async fn write_lines(
    child_stdin: &mut process::ChildStdin,
    lines: &str,
) -> io::Result<()> {
    child_stdin.write_all(lines.as_bytes()).await?;
    child_stdin.flush().await
}
