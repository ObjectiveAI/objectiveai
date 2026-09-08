//! Launching the subprocess, and the chunk stream it becomes.

use std::io;
use std::process::Stdio;

use diverge_provider_sdk::container_proxy::agent::enqueue::Fate;
use diverge_provider_sdk::shared::containers::run_loop::response::{
    AgenticLoopChunk, UserChunk,
};
use futures_util::Stream;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::agent::{Agent, Effort, Tools};
use crate::continuation;
use crate::response;

use super::error;
use super::install;
use super::pending;
use super::replies;
use super::session_id;
use super::stdin;
use super::writer;

/// Start the run: write the continuation's files if resuming, launch
/// Claude Code, hand it the prompt, and return the RUN AS A STREAM —
/// the reader is not a spawned task; whoever consumes the stream
/// drives it.
///
/// The caller ensures there is one run per container lifetime — the
/// same door guard the run handler owns — so this never contends
/// with an earlier subprocess.
///
/// The agent's knobs ride the argv: the model verbatim, thinking
/// on/off, effort 1:1 (all five of the SDK's tiers exist in current
/// Claude Code). The request's MOUNTS never reach this container at
/// all: mounting them is the SERVER's protocol obligation,
/// discharged before this container was even deployed — the run
/// simply finds them on disk (skills under the config dir's
/// `skills/`, agent definitions under `agents/`, wherever the
/// caller pointed them).
///
/// Each stdout line parses strictly per the [`response`] module's
/// contract and becomes the chunks it means — most records mean
/// none — with failures travelling as the [`error::Error`] they
/// are, for the consumer to judge.
pub async fn spawn(
    agent: Agent,
    continuation: Option<continuation::Continuation>,
    prompt: String,
) -> io::Result<
    impl Stream<Item = Result<AgenticLoopChunk, error::Error>> + Send,
> {
    // Before anything: Claude Code must EXIST. The run handler has
    // already checked with its own error body — this arm makes
    // spawning uninstalled impossible by construction, not by
    // call-site discipline.
    if let Err(error) = install::installed().await {
        return Err(io::Error::other(error.clone()));
    }

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
        .arg("--enable-auth-status")
        .arg("--dangerously-skip-permissions")
        // The proxy's MCP server, named by the container SDK so the
        // port and the path are spelled in one place.
        .arg("--mcp-config")
        .arg(
            serde_json::json!({
                "mcpServers": {
                    "diverge": {
                        "type": "http",
                        "url": diverge_container_proxy_sdk::mcp_url(),
                    }
                }
            })
            .to_string(),
        )
        .arg("--model")
        .arg(&agent.model)
        // The built-ins the model sees: the always-on names (so the
        // list is never empty) and then exactly the agent's `true`
        // switches. The `mcp__*` tools are not built-ins and ride
        // untouched. This is a different switch from
        // `--dangerously-skip-permissions` above: that one removes
        // the prompts, this one removes the tools.
        .arg("--tools")
        .arg(tools_flag(&agent.tools))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        // A dropped stream is an abandoned run: the runtime kills the
        // child, and the Teardown guard settles the rest.
        .kill_on_drop(true);
    match agent.thinking {
        // No flag when unsaid: Claude Code's own default is thinking
        // on, adaptive.
        None => {}
        Some(true) => {
            command.arg("--thinking").arg("enabled");
        }
        Some(false) => {
            command.arg("--thinking").arg("disabled");
        }
    }
    if let Some(effort) = agent.effort {
        command.arg("--effort").arg(effort_flag(effort));
    }
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
    Ok(read(child_stdout, child, reply_sender))
}

/// The `--effort` value for an SDK tier, 1:1.
/// The built-ins with no switch that are always on: the MCP
/// resource tools (the caller's resources, through the proxy),
/// `Skill`, and `ToolSearch` (how the model loads deferred tools'
/// schemas). `LSP` is the one always off.
const ALWAYS_ON: [&str; 6] = [
    "ListMcpResources",
    "ReadMcpResource",
    "ReadMcpResourceDir",
    "RefreshMcpTools",
    "Skill",
    "ToolSearch",
];

/// The `--tools` value: [`ALWAYS_ON`] and then the switched-on
/// names, comma-joined.
fn tools_flag(tools: &Tools) -> String {
    ALWAYS_ON
        .into_iter()
        .chain(tools.names())
        .collect::<Vec<_>>()
        .join(",")
}

fn effort_flag(effort: Effort) -> &'static str {
    match effort {
        Effort::Low => "low",
        Effort::Medium => "medium",
        Effort::High => "high",
        Effort::Xhigh => "xhigh",
        Effort::Max => "max",
    }
}

/// The run, as a stream: the one reader of stdout, and the child's
/// reaper — driven by whoever polls it, not a task of its own.
/// Progress therefore rides the consumer: a caller that stops
/// reading stalls the drain, and with it Claude Code (a full stdout
/// pipe blocks it) and the queue's fate resolution. That is the
/// caller stalling its own run; the module's deadlock audit is about
/// TASKS, and still holds — the consumer is the SSE response task,
/// independent of the queue verbs' handler tasks.
///
/// Per line: parse strictly, then two taps before conversion. A
/// `control_response` record goes, cloned, through the reply
/// sender — the receiver half sits behind [`replies::REPLIES`], so
/// only that lock's holder reads it. A replay echo whose uuid is in
/// [`pending::PENDING`] is a delivery: the fate resolves
/// `delivered` — strict fates' third arm — and the `user` chunk
/// carrying the prompt is yielded at exactly this position, which is
/// the protocol's: Claude Code yields the tool-response records
/// BEFORE the replay, so the mark lands behind the answers it
/// followed. A replay with no pending entry — the initial prompt's
/// echo, resumed history, a message dequeued mid-delivery — taps
/// nothing. The first record naming the session is also captured
/// into [`session_id::SESSION_ID`], the harvest's key. Then the
/// record converts, and every chunk is yielded; a line that failed
/// to parse is yielded as its error.
///
/// # Error-typed records travel as errors; FATALITY IS FINALITY
///
/// A rejected rate limit, a failed auth status, an error result, or
/// a line that failed the parse is yielded as an `Err` — always,
/// wherever it falls. This side never judges how bad it is, because
/// it cannot know yet: the CONSUMER decides by what follows. An
/// error before the run's first chunk is the request's own failure
/// (the status, the run handler's first-item contract); an error the run
/// outlives — a later chunk arrives — was survivable news; an error
/// the stream ends behind was the run's death. The error result
/// still bills: its `Err` is yielded first and its conversion
/// yields the usage chunk right behind it, so a billed failure is
/// never the stream's last word by construction. Deliberately NOT
/// error-typed: `api_retry` narration and the assistant `error`
/// markers — Claude Code's retry in flight, whose terminal verdict
/// arrives as the result record. Yielding an `Err` ends nothing on
/// this side — the drain, the fates and the reaping continue.
///
/// # The end, graceful or not
///
/// When stdout ends — EOF, or an IO error ends the same way — the
/// ORDER is load-bearing: the reply sender drops FIRST, so a dequeue
/// holding the locks mid-wait wakes on the closed channel and
/// resolves; then [`close`] clears the locks ONE AT A TIME — never
/// nested, so no cycle against the dequeue's joined hold — and
/// misses every fate still pending, which nothing can slip behind
/// (no writer, no registration); then the child is reaped, and the
/// stream's end is the caller's end-of-stream. A stream DROPPED
/// instead of drained gets the same settlement: the reply sender
/// dies with the generator's state, the runtime kills the child
/// (`kill_on_drop`), and the [`Teardown`] guard — constructed
/// OUTSIDE the generator, so even a never-polled stream carries
/// it — spawns [`close`] to settle the locks and the fates.
fn read(
    child_stdout: process::ChildStdout,
    child: process::Child,
    reply_sender: mpsc::UnboundedSender<
        response::control::ControlResponse,
    >,
) -> impl Stream<Item = Result<AgenticLoopChunk, error::Error>> + Send {
    // Outside the generator, deliberately: a stream dropped before
    // its first poll never runs a line of the body, but its captured
    // locals still drop.
    let teardown = Teardown;
    async_stream::stream! {
        let _teardown = teardown;
        let mut child = child;
        let reply_sender = reply_sender;
        let mut lines = BufReader::new(child_stdout).lines();
        // One buffer for the whole stream: each record's chunks land
        // here, drain as yields, and the allocation stays.
        let mut chunks: Vec<AgenticLoopChunk> = Vec::new();
        // Whether the session has been named — the capture's latch.
        let mut session_seen = false;
        while let Ok(Some(line)) = lines.next_line().await {
            let record = match serde_json::from_str::<
                response::StdoutMessage,
            >(&line)
            {
                Ok(record) => record,
                Err(parse) => {
                    yield Err(error::Error::Parse(parse));
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
                                Fate::Delivered,
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
            // The harvest's key: the first record naming the session.
            if !session_seen {
                let observed = match &record {
                    response::StdoutMessage::Assistant(assistant) => {
                        Some(assistant.session_id.as_str())
                    }
                    response::StdoutMessage::User(user) => {
                        user.session_id.as_deref()
                    }
                    response::StdoutMessage::Result(
                        response::result::Result::Success {
                            session_id,
                            ..
                        },
                    ) => Some(session_id.as_str()),
                    response::StdoutMessage::Result(
                        response::result::Result::Error(error),
                    ) => Some(error.session_id.as_str()),
                    _ => None,
                };
                if let Some(observed) = observed {
                    *session_id::SESSION_ID.lock().await =
                        Some(observed.to_string());
                    session_seen = true;
                }
            }
            // Error-typed records travel as errors, wherever they
            // fall — fatality is the consumer's, decided by what
            // follows. The error result alone falls through to
            // conversion too: its bill rides right behind its error.
            let record = match record {
                response::StdoutMessage::RateLimitEvent(event)
                    if event.rejected() =>
                {
                    yield Err(error::Error::RateLimit(event));
                    continue;
                }
                response::StdoutMessage::AuthStatus(status)
                    if status.failed() =>
                {
                    yield Err(error::Error::Auth(status));
                    continue;
                }
                response::StdoutMessage::Result(
                    response::result::Result::Error(result),
                ) => {
                    yield Err(error::Error::Result(result.clone()));
                    response::StdoutMessage::Result(
                        response::result::Result::Error(result),
                    )
                }
                record => record,
            };
            record.into_chunks(&mut chunks);
            for chunk in chunks.drain(..) {
                yield Ok(chunk);
            }
        }
        drop(reply_sender);
        close().await;
        let _ = child.wait().await;
    }
}

/// The settlement: locks cleared one at a time, every fate still
/// pending missed. Idempotent — the graceful end runs it inline and
/// [`Teardown`] runs it again.
async fn close() {
    *writer::WRITER.lock().await = None;
    *replies::REPLIES.lock().await = None;
    // The run is over: every fate still undecided is now decided.
    // Nothing registers behind this — registration needs the writer.
    let uuids: Vec<String> = pending::PENDING
        .iter()
        .map(|entry| entry.key().clone())
        .collect();
    for uuid in uuids {
        if let Some((_, fate)) = pending::PENDING.remove(&uuid) {
            let _ = fate.send(
                Fate::Missed,
            );
        }
    }
}

/// Settles the run when the stream drops, however it drops.
///
/// [`Drop`] cannot await, so the closing rides a spawned task; the
/// graceful path makes it a no-op. Constructed before the generator
/// and captured into it, so even a stream dropped unpolled — whose
/// body never ran a line — still settles.
struct Teardown;

impl Drop for Teardown {
    fn drop(&mut self) {
        tokio::spawn(close());
    }
}
