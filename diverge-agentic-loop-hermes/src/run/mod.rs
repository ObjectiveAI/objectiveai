//! Driving the run: the whole lifetime, as one stream of chunks.
//!
//! [`run`] owns everything between the request and the end: the
//! agent planned and its vault documents acquired ([`vault`]), the
//! filesystem laid down ([`filesystem::prepare`]), the session
//! restored or read ([`continuation::session`]), `hermes gateway`
//! spawned and awaited ([`Gateway`]), the turns driven over `/v1/runs`
//! ([`raw`]) and converted into chunks ([`Turn`]), the queue consulted
//! at the turn's end ([`QUEUE`]), the gateway stopped, the documents
//! read back and set ([`vault::Held::release`]), and the continuation
//! harvested into the rows ([`continuation::harvest`]).
//!
//! # The queue is consulted when a turn ends
//!
//! Hermes cannot be steered mid-turn: nothing reaches a run in flight.
//! So a message enqueued during a turn waits, and at the turn's end
//! the queue gets its look, atomically: an empty queue is closed in
//! the same lock hold that proved it empty, and the run ends;
//! messages pending are each answered `delivered`, yielded as a
//! `user` chunk, and — joined with a blank line between, the way
//! openrouter joins a run of prompts — become the next turn's input
//! on the same session, resumed. Invoking Hermes again IS the
//! delivery.
//!
//! # What ends a run, and what does not
//!
//! Before the gateway is up there is nothing to salvage, and a
//! failure is the stream's one [`Err`] — the request's own failure,
//! a status. From then on every failure — a turn that will not start,
//! a stream that breaks, `run.failed`, a document that will not set,
//! a harvest that cannot happen — is a fatal `notification` chunk,
//! and the run still stops the gateway and does what it can of the
//! way back up: the session on disk is the truth of what happened,
//! whatever the wire said.
//!
//! # One run at a time, and the settlement releases the lock
//!
//! The run holds the [`Claim`], handed in by the server, inside a
//! [`Teardown`] captured into the stream: when the stream drops —
//! finished, or abandoned by a caller that left — the teardown marks
//! the claim settling, closes the queue on a task, and only then
//! drops the claim. The gateway dies with the stream (`kill_on_drop`),
//! and held vault locks lapse by their TTL.

mod approval;
mod convert;
mod error;
mod gateway;
mod id;

pub mod raw;

pub use convert::*;
pub use error::*;
pub use gateway::*;

use std::sync::Arc;

use diverge_container_proxy_sdk::Client;
use diverge_container_proxy_sdk::agent::run::request::Message;
use diverge_provider_sdk::endpoints::containers::agents::run::server::response::{
    AgenticLoopChunk, user_parts,
};
use futures_util::{Stream, StreamExt as _};
use rmcp::model::ContentBlock;
use sqlx::PgPool;

use crate::agent::{Agent, Effort};
use crate::claim::Claim;
use crate::continuation;
use crate::filesystem;
use crate::queue::QUEUE;
use crate::response::Event;
use crate::vault;

/// The run, whole: one stream of chunks. `generation` is the queue's,
/// from [`QUEUE.open`](crate::queue::Queue::open); `claim` is the run
/// lock, released by the teardown.
pub fn run(
    client: Arc<Client>,
    pool: PgPool,
    agent: Agent,
    messages: Vec<Message>,
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

        let plan = match filesystem::plan(&agent).await {
            Ok(plan) => plan,
            Err(error) => {
                yield Err(Error::Prepare(error));
                return;
            }
        };
        let keys: Vec<&'static str> =
            plan.documents.iter().map(|document| document.key).collect();
        let (documents, held) = match vault::acquire(&client, &keys).await {
            Ok(acquired) => acquired,
            Err(error) => {
                yield Err(Error::Vault(error));
                return;
            }
        };
        let prepared = match filesystem::prepare(&plan, &documents).await {
            Ok(prepared) => prepared,
            Err(error) => {
                yield Err(Error::Prepare(error));
                return;
            }
        };
        let mut session = match continuation::session(&pool).await {
            Ok(session) => session,
            Err(error) => {
                yield Err(Error::Continuation(error));
                return;
            }
        };
        let mut gateway = match Gateway::start(&prepared.env) {
            Ok(gateway) => gateway,
            Err(error) => {
                yield Err(Error::Gateway(error));
                return;
            }
        };
        if let Err(error) = gateway.ready().await {
            yield Err(error);
            return;
        }

        let key = prepared.api_server_key;
        let model_options = model_options(&agent);
        let content: Vec<ContentBlock> = messages.iter().flat_map(|message| message.content.iter().cloned()).collect();
        let mut input = crate::content::render(&content);
        let mut started_on = messages;

        loop {
            let started = raw::run(
                &key,
                &raw::Request {
                    input,
                    instructions: agent.system_prompt.clone(),
                    session_id: session.clone(),
                    model: None,
                    model_options: model_options.clone(),
                },
            )
            .await;
            let run = match started {
                Ok(run) => run,
                Err(error) => {
                    yield Ok(notification(
                        serde_json::json!({ "kind": "start", "error": error.to_string() }),
                        true,
                    ));
                    break;
                }
            };
            // The messages the run started on, as the stream's first
            // chunks: their parts, each under its key, before the
            // harness says a word. The turn is on the wire; a start
            // that failed was the request's own, above.
            for message in started_on.drain(..) {
                for chunk in user_parts(&message.key, message.content) {
                    yield Ok(chunk);
                }
            }
            // A fresh run's session is named after the run.
            if session.is_none() {
                session = Some(run.run_id.clone());
            }

            let mut turn = Turn::default();
            let mut events = Box::pin(run.events);
            while let Some(event) = events.next().await {
                let event = match event {
                    Ok(event) => event,
                    Err(error) => {
                        yield Ok(notification(
                            serde_json::json!({ "kind": "stream", "error": error.to_string() }),
                            true,
                        ));
                        break;
                    }
                };
                if let Event::ApprovalRequest(_) = &event {
                    tokio::spawn(approval::approve(key.clone(), run.run_id.clone()));
                }
                for chunk in turn.convert(event) {
                    yield Ok(chunk);
                }
            }

            // The tip may have rotated under compaction: the
            // database says where the next turn records.
            if let Ok(Some(tip)) = filesystem::session().await {
                session = Some(tip);
            }

            // The turn's end: the queue's look, atomic. Empty closes
            // it and ends the run; pending opens another turn.
            let taken = QUEUE.take_or_close().await;
            if taken.is_empty() {
                break;
            }
            let mut blocks = Vec::new();
            for message in taken {
                for chunk in user_parts(&message.key, message.content.clone()) {
                    yield Ok(chunk);
                }
                blocks.extend(message.content.clone());
                message.deliver();
            }
            input = crate::content::render(&blocks);
        }

        if let Err(error) = gateway.stop().await {
            yield Ok(notification(
                serde_json::json!({ "kind": "gateway", "error": error.to_string() }),
                true,
            ));
        }

        // The way back up: the documents as the run left them, set
        // and unlocked; then the continuation into the rows.
        match filesystem::read_back(&plan).await {
            Ok(documents) => {
                if let Err(error) = held.release(documents).await {
                    yield Ok(notification(error.message(), true));
                }
            }
            Err(error) => {
                yield Ok(notification(
                    serde_json::json!({ "kind": "read_back", "error": error.to_string() }),
                    true,
                ));
            }
        }
        if let Err(error) = continuation::harvest(&pool).await {
            yield Ok(notification(error.message(), true));
        }
    }
}

/// The per-request model options `/v1/runs` reads, from the agent's
/// effort: `reasoning: {enabled, effort}` — `none` disables, the
/// other rungs name themselves, and an unsaid effort sends nothing.
fn model_options(agent: &Agent) -> Option<serde_json::Map<String, serde_json::Value>> {
    let effort = agent.effort?;
    let reasoning = match effort {
        Effort::None => serde_json::json!({ "enabled": false }),
        effort => serde_json::json!({
            "enabled": true,
            "effort": serde_json::to_value(effort).expect("a unit variant"),
        }),
    };
    let mut options = serde_json::Map::new();
    options.insert("reasoning".to_string(), reasoning);
    Some(options)
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
