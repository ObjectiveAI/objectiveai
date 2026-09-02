//! Driving the run: the whole lifetime, as one stream of the
//! container's own items.
//!
//! [`run`] owns everything between the request and the closer:
//! the filesystem laid down ([`filesystem::prepare`]), `hermes
//! gateway` spawned and awaited ([`Gateway`]), the turns driven over
//! `/v1/runs` ([`raw`]) and converted into chunks ([`Turn`]), the
//! enqueue queue consulted at its seams ([`QUEUE`]), the gateway
//! stopped, and the way back up ([`filesystem::finish`]) — the
//! resources, then the continuation, last.
//!
//! # The enqueue seams
//!
//! The proxy folds a queued prompt onto the next tool response and
//! tells this container nothing but the RETURN of its `/enqueue`
//! call, so the return is the signal: its moment is recorded, and
//! the prompt is yielded as a `user` chunk right after the tool
//! response whose completion is the first at or after that moment —
//! proxied calls are sequential barriers, so that completion is the
//! one. A prompt still pending when a turn ends is not lost: the
//! proxy is told to fold nothing stale, the prompt is answered
//! `delivered`, yielded as a `user` chunk opening the next turn, and
//! becomes that turn's input — the same session, resumed. An empty
//! queue at a turn's end closes the run.
//!
//! # What ends a run, and what does not
//!
//! Before the gateway is up there is nothing to salvage, and a
//! failure is the stream's one [`Err`]. From then on every failure —
//! a turn that will not start, a stream that breaks, `run.failed` —
//! is a fatal `notification` chunk, and the run still stops the
//! gateway and closes with what the database holds: the session is
//! the truth of what happened, whatever the wire said.

mod approval;
mod convert;
mod error;
mod gateway;
mod id;
mod item;
mod proxy;
mod queue;

pub mod raw;

pub use convert::*;
pub use error::*;
pub use gateway::*;
pub use item::*;
pub use queue::*;

use diverge_provider_sdk::agentic_loop_container::request::Request;
use diverge_provider_sdk::endpoints::agentic_loop::run::client::request::agent::Agent;
use diverge_provider_sdk::endpoints::agentic_loop::run::client::request::agent::hermes;
use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::{
    AgenticLoopChunk, UserChunk,
};
use futures_util::{Stream, StreamExt as _};

use crate::fetcher::Fetcher;
use crate::filesystem;
use crate::filesystem::PrepareError;
use crate::response::Event;

/// The run, whole: one stream of [`Item`]s, the continuation last.
pub fn run(
    request: &Request,
    fetcher: Fetcher,
) -> impl Stream<Item = Result<Item, Error>> {
    // The request's parts this needs, owned: the stream outlives
    // the borrow.
    let agent = match &request.agent {
        Agent::Hermes(agent) => Ok(agent.clone()),
        _ => Err(Error::Prepare(PrepareError::WrongAgent)),
    };
    let prompt = prompt_text(&request.prompt).ok_or(Error::Prompt);
    let request = request.clone();

    async_stream::stream! {
        // First, before anything can fail: the guard that closes the
        // queue however this ends.
        let _close = CloseOnDrop;

        let agent = match agent {
            Ok(agent) => agent,
            Err(error) => {
                yield Err(error);
                return;
            }
        };
        let mut input = match prompt {
            Ok(prompt) => prompt,
            Err(error) => {
                yield Err(error);
                return;
            }
        };
        let prepared = match filesystem::prepare(&request, fetcher).await {
            Ok(prepared) => prepared,
            Err(error) => {
                yield Err(Error::Prepare(error));
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
        let mut session = prepared.session;
        let model_options = model_options(&agent);

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
                    yield Ok(Item::Chunk(notification(
                        serde_json::json!({ "kind": "start", "error": error.to_string() }),
                        true,
                    )));
                    break;
                }
            };
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
                        yield Ok(Item::Chunk(notification(
                            serde_json::json!({ "kind": "stream", "error": error.to_string() }),
                            true,
                        )));
                        break;
                    }
                };
                if let Event::ApprovalRequest(_) = &event {
                    tokio::spawn(approval::approve(key.clone(), run.run_id.clone()));
                }
                let completed_at = match &event {
                    Event::ToolCompleted(completed) => Some(completed.timestamp),
                    _ => None,
                };
                for chunk in turn.convert(event) {
                    yield Ok(Item::Chunk(chunk));
                }
                if let Some(at) = completed_at {
                    for prompt in QUEUE.deliveries_before(at).await {
                        yield Ok(Item::Chunk(user(prompt)));
                    }
                }
            }

            // The tip may have rotated under compaction: the
            // database says where the next turn records.
            if let Ok(Some(tip)) = filesystem::session().await {
                session = Some(tip);
            }

            // The run's last look at the queue.
            let taken = QUEUE.take_or_close().await;
            if taken.is_empty() {
                break;
            }
            for prompt in &taken {
                yield Ok(Item::Chunk(user(prompt.clone())));
            }
            input = taken.join("\n\n");
        }

        // A fold whose completion never came — the run ended between
        // the two — is still a delivery, spoken at the end.
        for prompt in QUEUE.deliveries_before(f64::INFINITY).await {
            yield Ok(Item::Chunk(user(prompt)));
        }

        if let Err(error) = gateway.stop().await {
            yield Err(Error::Gateway(error));
            return;
        }

        let mut exports = Box::pin(filesystem::finish(&request));
        while let Some(export) = exports.next().await {
            match export {
                Ok(export) => yield Ok(Item::from(export)),
                Err(error) => {
                    yield Err(Error::Finish(error));
                    return;
                }
            }
        }
    }
}

/// The whole prompt as text: every block's text, joined by blank
/// lines — or [`None`] the moment any block is richer than text, or
/// when there are no blocks at all.
fn prompt_text(prompt: &[rmcp::model::ContentBlock]) -> Option<String> {
    if prompt.is_empty() {
        return None;
    }
    let mut texts = Vec::with_capacity(prompt.len());
    for block in prompt {
        texts.push(block.as_text()?.text.as_str());
    }
    Some(texts.join("\n\n"))
}

/// The per-request model options `/v1/runs` reads, from the agent's
/// effort: `reasoning: {enabled, effort}` — `none` disables, the
/// other rungs name themselves, and an unsaid effort sends nothing.
fn model_options(
    agent: &hermes::Agent,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    let effort = agent.effort?;
    let reasoning = match effort {
        hermes::Effort::None => serde_json::json!({ "enabled": false }),
        effort => serde_json::json!({
            "enabled": true,
            "effort": serde_json::to_value(effort).expect("a unit variant"),
        }),
    };
    let mut options = serde_json::Map::new();
    options.insert("reasoning".to_string(), reasoning);
    Some(options)
}

/// A `user` chunk: a queued prompt, at the position it landed.
fn user(prompt: String) -> AgenticLoopChunk {
    AgenticLoopChunk::User(UserChunk {
        r#type: Default::default(),
        prompt,
        meta: None,
    })
}
