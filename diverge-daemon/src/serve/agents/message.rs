//! A message to an agent.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::message::client::request;
use diverge_sdk::daemon::endpoints::agents::message::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::daemon::reference;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use std::sync::Arc;

use super::state;
use crate::containers::message;
use crate::daemon::{Arrival, Daemon};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, agents};

/// The agent a request is a message to, when it is one this daemon
/// serves: `None` for any other request, and for a message relayed on
/// — the daemon it is relayed to stamps it, as it reads it. Read by
/// the connection's read loop, which stamps it with this: see
/// [`Live::arriving`](crate::daemon::Live::arriving). Every other
/// request is told by its tag alone.
pub fn addressed(payload: &[u8]) -> Option<reference::Agent> {
    let frame = request::Frame::decode(payload).ok()?;
    frame.relay.is_none_or(|relay| relay.0.is_empty()).then_some(frame.agent)
}

/// Deliver the message and finish the scope: the answer, then the
/// finish. `arrival` is the read loop's stamp, which every message
/// served here carries; one that came without is stamped now.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, arrival: Option<Arrival>, who: Who, daemon: &Arc<Daemon>) {
    let arrival = match arrival {
        Some(arrival) => arrival,
        None => daemon.live.arriving(frame.agent.clone()).await,
    };
    let answer = serve(&scope, frame, arrival, who, daemon).await;
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `message` grant at all, or for an agent the
/// grants do not reach; the error for an agent that is not there, or
/// a container that could not be started; else the message sent to
/// the agent's container — started if it was not up — under a key
/// the daemon mints and the index it was stamped with as it was read,
/// the run then fed for its burst as [`message::enqueue`] states, the
/// scope open until its fate is known: `Delivered` once the agent's
/// program has it — the loop it started, or the program's own queue
/// — or the container's own error. A message that is not sent is
/// counted out all the same — [`message::refused`] — so its burst is
/// still fed. Nothing takes a message back: the client opens no
/// channel on the scope, and the answer is the fate.
async fn serve(scope: &ScopeHandle, frame: request::Frame, arrival: Arrival, who: Who, daemon: &Arc<Daemon>) -> Frame {
    let (agent, sender) = match judged(scope, &frame.agent, who, daemon).await {
        Ok(judged) => judged,
        Err(answer) => {
            message::refused(daemon, arrival).await;
            return answer;
        }
    };
    let fate = match message::enqueue(daemon, arrival, &agent, frame.content, sender).await {
        Ok(fate) => fate,
        Err(error) => {
            return Frame::Error(match (error.unsupported(), error.no_source()) {
                (Some(what), _) => reply::unsupported(what),
                (None, Some(what)) => reply::no_source(what),
                (None, None) => reply::failure(&error),
            });
        }
    };
    match fate.await {
        message::Fate::Delivered => Frame::Delivered,
        message::Fate::Error(error) => Frame::Error(error),
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
