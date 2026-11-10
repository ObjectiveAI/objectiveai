//! A message to an agent.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::message::client::request;
use diverge_sdk::daemon::endpoints::agents::message::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::daemon::reference;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use std::sync::Arc;

use super::state;
use crate::containers::message;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{Turn, reply};
use crate::store::{self, agents};

/// Deliver the message and finish the scope: the answer, then the
/// finish. `turn` is the request's place in the order its connection
/// sent it, which the message is admitted in.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, turn: Turn, who: Who, daemon: &Arc<Daemon>) {
    let answer = serve(&scope, frame, turn, who, daemon).await;
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `message` grant at all, or for an agent the
/// grants do not reach; the error for an agent that is not there; else
/// the message admitted to the agent's outbox, the scope open until
/// its fate is known: `Delivered` once the agent's program has it —
/// the loop it started, or the program's own queue — or the error: a
/// container that could not be started or activated, or the
/// container's own. Nothing takes a message back: the client opens no
/// channel on the scope, and the answer is the fate. The rules of the
/// admission, which make the messages one connection sends reach the
/// agent in the order it sent them:
///
/// 1. **The lookups first, and at once.** Who sends, the agent named
///    and what the grants make of it are read as the handler starts,
///    each message's beside every other's.
/// 2. **Then the turn.** The message waits for its [`Turn`] to come —
///    every request its connection sent before it let go, which for a
///    message is admitted — and is admitted to the outbox:
///    [`message::admit`]. The wait is only for the lookups of the
///    messages before it.
/// 3. **Then the turn is let go**, and only the fate is waited for. A
///    message refused lets its turn go by returning, as rule 4 of
///    [`Turn`] states.
/// 4. **A fate never said** — the agent's pump died with the message
///    waiting — is the daemon's error.
async fn serve(scope: &ScopeHandle, frame: request::Frame, mut turn: Turn, who: Who, daemon: &Arc<Daemon>) -> Frame {
    let (agent, sender) = match judged(scope, &frame.agent, who, daemon).await {
        Ok(judged) => judged,
        Err(answer) => return answer,
    };
    turn.come().await;
    let fate = message::admit(daemon, agent, frame.content, sender).await;
    drop(turn);
    match fate.await {
        Ok(message::Fate::Delivered) => Frame::Delivered,
        Ok(message::Fate::Error(error)) => Frame::Error(error),
        Err(_) => Frame::Error(reply::failure(&"the message's delivery ended before its fate was known")),
    }
}

/// The agent `reference` names, and who sends to it, once the grants
/// let the message through — the scope then told so; else the answer:
/// `Forbidden`, the error for an agent that is not there, or the
/// store's failure.
async fn judged(scope: &ScopeHandle, reference: &reference::Agent, who: Who, daemon: &Arc<Daemon>) -> Result<(agents::Agent, Creator), Frame> {
    let failed = |error: store::Error| Frame::Error(reply::failure(&error));
    let mut conn = daemon.store.acquire().await.map_err(failed)?;
    let Some(standing) = Standing::of(&mut conn, who).await.map_err(failed)? else {
        return Err(Frame::Forbidden);
    };
    if !judge::agents::holds(&standing, Over::Message) {
        return Err(Frame::Forbidden);
    }
    let Some(agent) = agents::by_reference(&mut conn, reference, false).await.map_err(failed)? else {
        return Err(Frame::Error(reply::failure(&"the agent named is none the daemon has")));
    };
    drop(conn);
    if !judge::agents::over(&standing, Over::Message, &agent, state(daemon, agent.id).await) {
        return Err(Frame::Forbidden);
    }
    scope.allowed().await;
    Ok((agent, standing.creator.clone()))
}
