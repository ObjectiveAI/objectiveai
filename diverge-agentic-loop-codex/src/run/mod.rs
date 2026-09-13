//! Driving the run: the whole lifetime, as one stream of chunks.
//!
//! [`run`] owns everything between the request and the end: the
//! login resolved ([`Auth`]), `config.toml` written ([`config`]),
//! the continuation loaded and restored once ([`continuation`]),
//! `codex exec --json` spawned per turn ([`Process`]) and its events
//! converted into chunks ([`Turn`]), the queue consulted at each
//! turn's end ([`QUEUE`]), the rollouts harvested into the rows.
//!
//! # The queue is consulted when a turn ends
//!
//! A turn is one `codex exec` process, and nothing steers it: a
//! message enqueued during a turn waits, and at the turn's end the
//! queue gets its look, atomically — an empty queue is closed in the
//! same lock hold that proved it empty, and the run ends; messages
//! pending are each answered `delivered`, yielded as a `user` chunk,
//! and, joined with a blank line between, become the next turn's
//! prompt on the same thread, resumed. Invoking Codex again IS the
//! delivery.
//!
//! # What ends a run, and what does not
//!
//! Before the first process has written a single event there is
//! nothing to salvage, and a failure is the stream's one [`Err`] —
//! the request's own failure, a status. From then on every failure —
//! a turn that fails, a process that dies or is interrupted, a
//! harvest that cannot happen — is a fatal `notification` chunk, and
//! the run still does what it can of
//! the way back up: the rollouts on disk are the truth of what
//! happened, and they are harvested whenever a thread is known.
//!
//! # One run at a time, and the settlement releases the lock
//!
//! The run holds the [`Claim`], handed in by the server, inside a
//! [`Teardown`] captured into the stream: when the stream drops —
//! finished, or abandoned by a caller that left — the teardown marks
//! the claim settling, closes the queue on a task, and only then
//! drops the claim. The process dies with the stream
//! (`kill_on_drop`).

mod convert;
mod error;
mod process;

pub use convert::*;
pub use error::*;
pub use process::*;

use std::collections::BTreeMap;
use std::sync::Arc;

use diverge_container_proxy_sdk::Client;
use diverge_provider_sdk::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use futures_util::Stream;
use sqlx::PgPool;

use crate::agent::Agent;
use crate::auth::{self, Auth};
use crate::claim::Claim;
use crate::config;
use crate::continuation;
use crate::queue::QUEUE;

/// The run, whole: one stream of chunks. `generation` is the queue's,
/// from [`QUEUE.open`](crate::queue::Queue::open); `claim` is the run
/// lock, released by the teardown.
pub fn run(
    client: Arc<Client>,
    pool: PgPool,
    agent: Agent,
    prompt: String,
    generation: u64,
    claim: Claim,
) -> impl Stream<Item = Result<AgenticLoopChunk, Error>> {
    // Outside the generator, deliberately: a stream dropped before
    // its first poll never runs a line of the body, but its captured
    // locals still drop.
    let teardown = Teardown {
        claim: Some(claim),
        generation,
    };

    async_stream::stream! {
        let _teardown = teardown;

        let login = match Auth::resolve(&client).await {
            Ok(login) => login,
            Err(error) => {
                yield Err(Error::Auth(error));
                return;
            }
        };
        let contents = match config::render(&agent) {
            Ok(contents) => contents,
            Err(error) => {
                yield Err(Error::Config(std::io::Error::other(error)));
                return;
            }
        };
        if let Err(error) = config::write(&contents).await {
            yield Err(Error::Config(error));
            return;
        }
        let mut thread = match continuation::load(&pool).await {
            Ok(thread) => thread,
            Err(error) => {
                yield Err(Error::Continuation(error));
                return;
            }
        };

        let mut env: BTreeMap<String, String> = BTreeMap::new();
        env.insert("CODEX_HOME".to_string(), auth::CODEX_HOME.to_string());
        if let Some((key, value)) = login.env() {
            env.insert(key.to_string(), value.to_string());
        }

        // Whether any event has been written: before the first, a
        // failure is the request's own; after, a fatal notification.
        let mut spoke = false;
        let mut input = prompt;

        'turns: loop {
            let mut process = match Process::start(&env, thread.thread_id.as_deref(), &input).await {
                Ok(process) => process,
                Err(error) => {
                    if spoke {
                        yield Ok(notification(
                            serde_json::json!({
                                "kind": "codex",
                                "error": format!("codex could not be spawned: {error}"),
                            }),
                            true,
                        ));
                        break;
                    }
                    yield Err(Error::Spawn(error));
                    return;
                }
            };

            let mut turn = Turn::new(thread.usage.clone());
            let mut broke = false;
            loop {
                match process.next().await {
                    Ok(Some(event)) => {
                        spoke = true;
                        for chunk in turn.convert(event) {
                            yield Ok(chunk);
                        }
                    }
                    Ok(None) => break,
                    Err(LineError::Parse { line, error }) => {
                        if !spoke {
                            yield Err(Error::Line(LineError::Parse { line, error }));
                            return;
                        }
                        yield Ok(notification(
                            serde_json::json!({
                                "kind": "protocol",
                                "error": format!("codex wrote a line that is not an event ({error}): {line}"),
                            }),
                            false,
                        ));
                    }
                    Err(LineError::Io(error)) => {
                        if !spoke {
                            yield Err(Error::Spawn(error));
                            return;
                        }
                        yield Ok(notification(
                            serde_json::json!({
                                "kind": "codex",
                                "error": format!("codex's pipe failed: {error}"),
                            }),
                            true,
                        ));
                        broke = true;
                        break;
                    }
                }
            }

            // The exit: nothing at all is the request's own failure;
            // a turn begun and never ended is an interruption; a
            // non-zero exit after a terminal event is news.
            let status = process.wait().await;
            if !spoke {
                yield Err(match status {
                    Ok(status) => Error::Exited(status),
                    Err(error) => Error::Spawn(error),
                });
                return;
            }
            if let Some(thread_id) = turn.thread_id.take() {
                thread.thread_id = Some(thread_id);
            }
            thread.usage = turn.baseline.clone();
            let mut fatal = broke || turn.failed;
            if !fatal && turn.started && !turn.terminal {
                yield Ok(notification(
                    serde_json::json!({
                        "kind": "interrupted",
                        "error": "the turn ended with neither turn.completed nor turn.failed",
                    }),
                    true,
                ));
                fatal = true;
            }
            match status {
                Ok(status) if status.success() || fatal => {}
                Ok(status) => {
                    yield Ok(notification(
                        serde_json::json!({
                            "kind": "codex",
                            "error": format!("codex exited with {status}"),
                        }),
                        false,
                    ));
                }
                Err(error) => {
                    yield Ok(notification(
                        serde_json::json!({
                            "kind": "codex",
                            "error": format!("codex could not be reaped: {error}"),
                        }),
                        false,
                    ));
                }
            }

            if fatal {
                break 'turns;
            }

            // The turn's end: the queue's look, atomic. Empty closes
            // it and ends the run; pending opens another turn.
            let taken = QUEUE.take_or_close().await;
            if taken.is_empty() {
                break;
            }
            let mut prompts = Vec::with_capacity(taken.len());
            for message in taken {
                yield Ok(user(message.prompt.clone()));
                prompts.push(message.prompt.clone());
                message.deliver();
            }
            input = prompts.join("\n\n");
        }

        // The way back up: the rollouts into the rows, whenever a
        // thread is known — the files are the truth of what happened,
        // however the run ended.
        if thread.thread_id.is_some() {
            if let Err(error) = continuation::harvest(&pool, &thread).await {
                yield Ok(notification(error.message(), true));
            }
        }
    }
}

/// Settles the run when the stream drops, however it drops — and
/// THEN releases the run lock.
///
/// [`Drop`] cannot await, so the queue's closing rides a spawned
/// task; the graceful path makes it a no-op, and the close carries
/// the run's generation. The [`Claim`] rides with it and drops on
/// that task's next line: the next run can only open once this one's
/// queue is closed, and a request that lands meanwhile waits for it
/// rather than being refused, because the claim is marked settling
/// before the task is spawned.
struct Teardown {
    claim: Option<Claim>,
    generation: u64,
}

impl Drop for Teardown {
    fn drop(&mut self) {
        let claim = self.claim.take();
        if let Some(claim) = &claim {
            claim.settling();
        }
        let generation = self.generation;
        tokio::spawn(async move {
            QUEUE.close(generation).await;
            drop(claim);
        });
    }
}
