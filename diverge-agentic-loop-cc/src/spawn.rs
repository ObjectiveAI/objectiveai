//! The Claude Code subprocess, and the tasks that speak to it.
//!
//! One run is one subprocess, and everything the harness says to it
//! goes through ONE writer task that owns stdin and the pending-fate
//! map outright. Enqueues, dequeues, the drain's replay sightings and
//! the end-of-stream close all arrive as [`Command`]s on a single
//! channel, each carrying its reply wire where it needs one — so a
//! single consumer serializes every write and every fate, and no lock
//! is ever held across an await.
//!
//! The drain task owns the other direction: it reads stdout line by
//! line, forwards every parse result to the caller's sender, and taps
//! the replay echoes — a `user` record with `isReplay: true` — which
//! are how Claude Code says an enqueued message entered the
//! conversation. The tap only REPORTS the uuid; resolving the fate is
//! the writer's, like every other pending-map touch.

use std::collections::HashMap;
use std::io;
use std::process::Stdio;

use diverge_provider_sdk::agentic_loop_container;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process;
use tokio::sync::{Mutex, mpsc, oneshot};
use uuid::Uuid;

use crate::continuation;
use crate::response;
use crate::stdin;

/// The way in: the writer task's channel, set once by [`spawn`].
///
/// `None` until a run starts — an enqueue before that is missed, a
/// dequeue finds nothing — and never taken back out: after the
/// subprocess ends, the writer itself answers with the closed-queue
/// fates, which keeps the close atomic with the map that proves it.
static COMMANDS: Mutex<Option<mpsc::UnboundedSender<Command>>> =
    Mutex::const_new(None);

/// Everything the writer task can be told, each entry carrying its
/// reply wire where one is owed.
enum Command {
    /// Queue a message for the running session.
    Enqueue {
        /// The message's text.
        prompt: String,
        /// Where its fate goes.
        fate: oneshot::Sender<agentic_loop_container::enqueue::Response>,
    },
    /// Withdraw everything still queued.
    Dequeue {
        /// Where the clearing's summary goes.
        reply: oneshot::Sender<agentic_loop_container::dequeue::Response>,
    },
    /// The drain saw a replay echo: the message with this uuid
    /// entered the conversation.
    Replayed {
        /// The echoed uuid.
        uuid: String,
    },
    /// The drain saw stdout end: the run is over.
    Closed,
}

/// Start the run: write the continuation's files if resuming, launch
/// Claude Code, hand it the prompt, and leave the writer and drain
/// tasks running.
///
/// The caller ensures there is one run per container lifetime — the
/// same door guard the root handler owns — so this never contends
/// with an earlier subprocess.
///
/// Every line Claude Code writes reaches `sender` as its parse
/// result, strict per the [`response`] module's contract; the
/// receiver's death stops nothing, because the drain's other duty —
/// reporting replays — outlives any listener.
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
    // the print prompt is the same line an enqueue writes. No pending
    // fate — its replay taps nothing, harmlessly.
    write_line(
        &mut child_stdin,
        &serde_json::to_string(&stdin::UserMessage {
            r#type: Default::default(),
            message: stdin::UserMessageBody {
                role: Default::default(),
                content: prompt,
            },
            uuid: Uuid::new_v4().to_string(),
        })
        .expect("a stdin line is plain structs and serializes"),
    )
    .await?;

    let (commands, receiver) = mpsc::unbounded_channel();
    *COMMANDS.lock().await = Some(commands.clone());
    tokio::spawn(writer(child_stdin, receiver));
    tokio::spawn(drain(child_stdout, child, commands, sender));
    Ok(())
}

/// Queue a message for the running session, and get the wire its
/// fate will arrive on.
///
/// No run yet, or no run anymore: the fate is already known — the
/// message is missed — and the returned receiver resolves
/// immediately.
pub async fn enqueue(
    prompt: String,
) -> oneshot::Receiver<agentic_loop_container::enqueue::Response> {
    let (fate, receiver) = oneshot::channel();
    match COMMANDS.lock().await.as_ref() {
        Some(commands) => {
            // The writer outlives every sender COMMANDS holds, so
            // this send cannot fail; if it somehow did, the fate
            // rides back inside the error and is missed honestly.
            if let Err(mpsc::error::SendError(Command::Enqueue {
                fate,
                ..
            })) = commands.send(Command::Enqueue { prompt, fate })
            {
                let _ = fate.send(
                    agentic_loop_container::enqueue::Response::Missed {
                        r#type: Default::default(),
                    },
                );
            }
        }
        None => {
            let _ = fate.send(
                agentic_loop_container::enqueue::Response::Missed {
                    r#type: Default::default(),
                },
            );
        }
    }
    receiver
}

/// Withdraw everything still queued.
///
/// No run, no queue: the naive answer is empty, and it is also the
/// honest one — a session that never started or already ended holds
/// nothing to withdraw.
pub async fn dequeue() -> agentic_loop_container::dequeue::Response {
    let (reply, receiver) = oneshot::channel();
    {
        // Scoped: the reply is awaited OUTSIDE the lock, so enqueues
        // keep flowing while the writer works.
        match COMMANDS.lock().await.as_ref() {
            Some(commands) => {
                if commands.send(Command::Dequeue { reply }).is_err() {
                    return agentic_loop_container::dequeue::Response::Empty {
                        r#type: Default::default(),
                    };
                }
            }
            None => {
                return agentic_loop_container::dequeue::Response::Empty {
                    r#type: Default::default(),
                };
            }
        }
    }
    receiver.await.unwrap_or(
        agentic_loop_container::dequeue::Response::Empty {
            r#type: Default::default(),
        },
    )
}

/// The writer task: the one owner of stdin and the pending map.
///
/// `pending.len()` is the queue ticker — it rises as enqueues land,
/// drops as replays resolve them, and zeroes on a dequeue or the
/// close. Commands are processed strictly in arrival order, which is
/// what makes the ordering free: a [`Command::Replayed`] can only
/// exist after this task itself wrote the message it echoes, so the
/// tap always finds its entry — unless a dequeue withdrew it first,
/// in which case ignoring the echo is the accepted design.
async fn writer(
    mut child_stdin: process::ChildStdin,
    mut commands: mpsc::UnboundedReceiver<Command>,
) {
    let mut pending: HashMap<
        String,
        oneshot::Sender<agentic_loop_container::enqueue::Response>,
    > = HashMap::new();
    let mut open = true;
    while let Some(command) = commands.recv().await {
        match command {
            Command::Enqueue { prompt, fate } => {
                if !open {
                    let _ = fate.send(
                        agentic_loop_container::enqueue::Response::Missed {
                            r#type: Default::default(),
                        },
                    );
                    continue;
                }
                let uuid = Uuid::new_v4().to_string();
                let line = serde_json::to_string(&stdin::UserMessage {
                    r#type: Default::default(),
                    message: stdin::UserMessageBody {
                        role: Default::default(),
                        content: prompt,
                    },
                    uuid: uuid.clone(),
                })
                .expect("a stdin line is plain structs and serializes");
                match write_line(&mut child_stdin, &line).await {
                    Ok(()) => {
                        pending.insert(uuid, fate);
                    }
                    // A broken stdin is the process dying: everything
                    // waiting, this message included, is missed.
                    Err(_) => {
                        open = false;
                        let _ = fate.send(
                            agentic_loop_container::enqueue::Response::Missed {
                                r#type: Default::default(),
                            },
                        );
                        for (_, fate) in pending.drain() {
                            let _ = fate.send(
                                agentic_loop_container::enqueue::Response::Missed {
                                    r#type: Default::default(),
                                },
                            );
                        }
                    }
                }
            }
            Command::Dequeue { reply } => {
                // Covers the closed writer too: closing drained the
                // map, and an empty queue is what empty means.
                if pending.is_empty() {
                    let _ = reply.send(
                        agentic_loop_container::dequeue::Response::Empty {
                            r#type: Default::default(),
                        },
                    );
                    continue;
                }
                // One buffered write for all the withdrawals; the
                // command going through IS the success, per design —
                // no waiting on control responses.
                let mut lines = String::new();
                for uuid in pending.keys() {
                    lines.push_str(
                        &serde_json::to_string(&stdin::ControlRequest {
                            r#type: Default::default(),
                            request_id: Uuid::new_v4().to_string(),
                            request: stdin::CancelAsyncMessage {
                                subtype: Default::default(),
                                message_uuid: uuid.clone(),
                            },
                        })
                        .expect(
                            "a stdin line is plain structs and serializes",
                        ),
                    );
                    lines.push('\n');
                }
                match write_all(&mut child_stdin, &lines).await {
                    Ok(()) => {
                        for (_, fate) in pending.drain() {
                            let _ = fate.send(
                                agentic_loop_container::enqueue::Response::Dequeued {
                                    r#type: Default::default(),
                                },
                            );
                        }
                        let _ = reply.send(
                            agentic_loop_container::dequeue::Response::Dequeued {
                                r#type: Default::default(),
                            },
                        );
                    }
                    Err(_) => {
                        open = false;
                        for (_, fate) in pending.drain() {
                            let _ = fate.send(
                                agentic_loop_container::enqueue::Response::Missed {
                                    r#type: Default::default(),
                                },
                            );
                        }
                        let _ = reply.send(
                            agentic_loop_container::dequeue::Response::Empty {
                                r#type: Default::default(),
                            },
                        );
                    }
                }
            }
            Command::Replayed { uuid } => {
                // Absence is a message dequeued mid-delivery, echoing
                // anyway — answered already, ignored by design.
                if let Some(fate) = pending.remove(&uuid) {
                    let _ = fate.send(
                        agentic_loop_container::enqueue::Response::Delivered {
                            r#type: Default::default(),
                        },
                    );
                }
            }
            Command::Closed => {
                open = false;
                for (_, fate) in pending.drain() {
                    let _ = fate.send(
                        agentic_loop_container::enqueue::Response::Missed {
                            r#type: Default::default(),
                        },
                    );
                }
            }
        }
    }
}

/// The drain task: the one reader of stdout, and the child's reaper.
///
/// Reads until the stream ends — an IO error on the pipe ends the
/// same way EOF does — then tells the writer the run is over and
/// waits the child to reap it. Dropping `sender` at the end is the
/// caller's end-of-stream.
async fn drain(
    child_stdout: process::ChildStdout,
    mut child: process::Child,
    commands: mpsc::UnboundedSender<Command>,
    sender: mpsc::UnboundedSender<
        Result<response::StdoutMessage, serde_json::Error>,
    >,
) {
    let mut lines = BufReader::new(child_stdout).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let parsed =
            serde_json::from_str::<response::StdoutMessage>(&line);
        // The fate tap: a replay echo means the enqueued message with
        // that uuid entered the conversation. Report it; the writer
        // resolves it.
        if let Ok(response::StdoutMessage::User(user)) = &parsed {
            if user.is_replay == Some(true) {
                if let Some(uuid) = &user.uuid {
                    let _ = commands.send(Command::Replayed {
                        uuid: uuid.clone(),
                    });
                }
            }
        }
        let _ = sender.send(parsed);
    }
    let _ = commands.send(Command::Closed);
    let _ = child.wait().await;
}

/// Write one NDJSON line: the serialized record, a newline, a flush.
async fn write_line(
    child_stdin: &mut process::ChildStdin,
    line: &str,
) -> io::Result<()> {
    child_stdin.write_all(line.as_bytes()).await?;
    child_stdin.write_all(b"\n").await?;
    child_stdin.flush().await
}

/// Write an already-newline-terminated batch, then flush once.
async fn write_all(
    child_stdin: &mut process::ChildStdin,
    lines: &str,
) -> io::Result<()> {
    child_stdin.write_all(lines.as_bytes()).await?;
    child_stdin.flush().await
}
