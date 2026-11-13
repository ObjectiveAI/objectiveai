//! Asking an agent something: a chat message, or a structured
//! decision.
//!
//! One request, of two kinds, which share who may send and how the
//! agent is found — [`judged`] — and part ways at what the agent's
//! image's document is held to: [`chat`], [`structured_decisions`].
//! Both admit to the agent's one outbox, in the order their connection
//! sent them. Unwatched, the answer is the fate; watched, it is
//! [`watch`]'s.

use std::sync::Arc;

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::message::client::request;
use diverge_sdk::daemon::endpoints::agents::message::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::daemon::reference;
use diverge_sdk::shared::containers::info::Document;
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::state;
use crate::containers::message;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{Turn, reply};
use crate::store::{self, agents, agents_templates, image_info};

pub mod chat;
pub mod structured_decisions;
pub mod watch;

/// Serve the request and finish the scope. `turn` is the request's
/// place in the order its connection sent it, which the message is
/// admitted in.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, turn: Turn, who: Who, daemon: &Arc<Daemon>) {
    serve(&scope, frame, turn, who, daemon).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `message` grant at all, or for an agent the
/// grants do not reach; the error for an agent that is not there; the
/// refusal the agent's image's document makes of the request, by its
/// kind — [`chat::held`], [`structured_decisions::held`]; else the
/// request admitted to the agent's outbox, and answered: unwatched,
/// with its fate — `Delivered` once the agent's program has it, or
/// the error: a container that could not be started or fed, or the
/// container's own — and watched, as [`watch::watch`] states. Nothing
/// takes a message back. The rules of the admission, which make the
/// messages one connection sends reach the agent in the order it sent
/// them:
///
/// 1. **The lookups first, and at once.** Who sends, the agent named
///    and what the grants make of it are read as the handler starts,
///    each message's beside every other's.
/// 2. **Then the turn.** The message waits for its [`Turn`] to come —
///    every request its connection sent before it let go, which for a
///    message is admitted — and is admitted to the outbox:
///    [`message::admit`]. The wait is only for the lookups of the
///    messages before it.
/// 3. **Then the turn is let go**, and only what comes of the message
///    is waited for. A message refused lets its turn go by returning,
///    as rule 4 of [`Turn`] states.
/// 4. **A fate never said** — the agent's pump died with the message
///    waiting — is the daemon's error.
async fn serve(scope: &ScopeHandle, frame: request::Frame, mut turn: Turn, who: Who, daemon: &Arc<Daemon>) {
    let (agent, sender, document) = match judged(scope, &frame.agent, who, daemon).await {
        Ok(judged) => judged,
        Err(Refused::Forbidden) => return reply::reply(scope, &Frame::Forbidden).await,
        Err(Refused::Error(error)) => return reply::reply(scope, &Frame::Error(error)).await,
    };
    let (held, what) = match frame.message {
        request::Message::Chat { content } => (chat::held(&agent, document.as_ref(), &content), message::What::Chat(content)),
        request::Message::StructuredDecisions { content, questions } => (
            structured_decisions::held(&agent, document.as_ref(), &content, &questions),
            message::What::Decision { content, questions },
        ),
    };
    if let Err(answer) = held {
        return reply::reply(scope, &answer).await;
    }
    let id = agent.id;
    // Subscribed before the message is admitted, and the handle let
    // go: nothing the log takes of the message is missed, and the
    // agent's deletion — the sender gone — ends the watch.
    let latest = match frame.watch {
        Some(true) => Some(daemon.live.log(&daemon.store, id).await.latest.subscribe()),
        _ => None,
    };
    turn.come().await;
    let admitted = message::admit(daemon, agent, what, sender).await;
    drop(turn);
    match latest {
        Some(latest) => watch::watch(scope, daemon, id, admitted, latest).await,
        None => {
            let answer = match admitted.fate.await {
                Ok(message::Fate::Delivered) => Frame::Delivered,
                Ok(message::Fate::Error(error)) => Frame::Error(error),
                Err(_) => Frame::Error(reply::failure(&"the message's delivery ended before its fate was known")),
            };
            reply::reply(scope, &answer).await;
        }
    }
}

/// Why the agent was not found for the sender: the answer either
/// request gives, in its own frame.
pub enum Refused {
    /// No `message` grant at all, or none reaching the agent.
    Forbidden,
    /// The agent is not there, or the store failed.
    Error(Error),
}

/// The agent `reference` names, who sends to it, and the document on
/// record for its template's image — `None` when the template is gone
/// or its image has none on record — once the grants let the request
/// through, the scope then told so; else why not. The grant is
/// `message` over agents, for a chat message and a structured decision
/// alike.
async fn judged(
    scope: &ScopeHandle,
    reference: &reference::Agent,
    who: Who,
    daemon: &Arc<Daemon>,
) -> Result<(agents::Agent, Creator, Option<Document>), Refused> {
    let failed = |error: store::Error| Refused::Error(reply::failure(&error));
    let mut conn = daemon.store.acquire().await.map_err(failed)?;
    let Some(standing) = Standing::of(&mut conn, who).await.map_err(failed)? else {
        return Err(Refused::Forbidden);
    };
    if !judge::agents::holds(&standing, Over::Message) {
        return Err(Refused::Forbidden);
    }
    let Some(agent) = agents::by_reference(&mut conn, reference, false).await.map_err(failed)? else {
        return Err(Refused::Error(reply::failure(&"the agent named is none the daemon has")));
    };
    let document = match agents_templates::by_id(&mut conn, &agent.template, false).await.map_err(failed)? {
        Some(record) => image_info::by_digest(&mut conn, &record.template.image.digest).await.map_err(failed)?,
        None => None,
    };
    drop(conn);
    if !judge::agents::over(&standing, Over::Message, &agent, state(daemon, agent.id).await) {
        return Err(Refused::Forbidden);
    }
    scope.allowed().await;
    Ok((agent, standing.creator.clone(), document))
}

/// The `image` error for an agent whose template is gone, or whose
/// image has no document on record: what either kind is answered
/// before its document is read.
fn undocumented(agent: &agents::Agent) -> Error {
    reply::image(&format!("the agent's template `{}` is gone, or its image has no document on record", agent.template))
}
