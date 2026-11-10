//! A message to an agent.

use diverge_sdk::daemon::endpoints::agents::message::client::request;
use diverge_sdk::daemon::endpoints::agents::message::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use std::sync::Arc;

use super::state;
use crate::containers::message;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, agents};

/// Deliver the message and finish the scope: the answer, then the
/// finish.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(&scope, frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `message` grant at all, or for an agent the
/// grants do not reach; the error for an agent that is not there, or
/// a container that could not be started; else the message sent to
/// the agent's container — started if it was not up — under a key
/// the daemon mints, in arrival order, the run then fed for its burst
/// as [`message::enqueue`] states, the scope open until its fate is
/// known: `Delivered` once the agent's program has it — the loop it
/// started, or the program's own queue — or the container's own
/// error. Nothing takes a message back: the client opens no channel
/// on the scope, and the answer is the fate.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::agents::holds(&standing, Over::Message) {
        return Ok(Frame::Forbidden);
    }
    let Some(agent) = agents::by_reference(&mut conn, &frame.agent, false).await? else {
        return Ok(Frame::Error(reply::failure(&"the agent named is none the daemon has")));
    };
    drop(conn);
    if !judge::agents::over(&standing, Over::Message, &agent, state(daemon, agent.id).await) {
        return Ok(Frame::Forbidden);
    }
    scope.allowed().await;
    let sender = standing.creator.clone();
    let fate = match message::enqueue(daemon, &agent, frame.content, sender).await {
        Ok(fate) => fate,
        Err(error) => {
            return Ok(Frame::Error(match (error.unsupported(), error.no_source()) {
                (Some(what), _) => reply::unsupported(what),
                (None, Some(what)) => reply::no_source(what),
                (None, None) => reply::failure(&error),
            }));
        }
    };
    Ok(match fate.await {
        message::Fate::Delivered => Frame::Delivered,
        message::Fate::Error(error) => Frame::Error(error),
    })
}
