//! A message to an agent.

use diverge_sdk::daemon::endpoints::agents::message::client::channel_request;
use diverge_sdk::daemon::endpoints::agents::message::client::request;
use diverge_sdk::daemon::endpoints::agents::message::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use std::sync::Arc;

use super::state;
use crate::containers::{self, message};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, agents};

/// Deliver the message, or see it cancelled, and finish the scope: the
/// answer, then the finish — or, for a cancel in time, the finish alone.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(&scope, frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Some(Frame::Error(reply::failure(&error))),
    };
    if let Some(answer) = answer {
        reply::reply(&scope, &answer).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `message` grant at all, or for an agent the
/// grants do not reach; the error for an agent that is not there, or
/// a container that could not be started; else the agent's container
/// started if it was not up, activated for the message — its secrets
/// taken and fed, which failing is the error — and the message
/// enqueued under a key the daemon mints, the scope open until its
/// fate is known: `Delivered` as the loop takes it, or the container's
/// own error; `None` — no answer, the finish alone — when the client's
/// one channel took it back first.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Option<Frame>, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Some(Frame::Forbidden));
    };
    if !judge::agents::holds(&standing, Over::Message) {
        return Ok(Some(Frame::Forbidden));
    }
    let Some(agent) = agents::by_reference(&mut conn, &frame.agent, false).await? else {
        return Ok(Some(Frame::Error(reply::failure(&"the agent named is none the daemon has"))));
    };
    drop(conn);
    if !judge::agents::over(&standing, Over::Message, &agent, state(daemon, agent.id).await) {
        return Ok(Some(Frame::Forbidden));
    }
    scope.allowed().await;
    let run = match containers::agent(daemon, &agent).await {
        Ok(run) => run,
        Err(error) => {
            return Ok(Some(Frame::Error(match (error.unsupported(), error.no_source()) {
                (Some(what), _) => reply::unsupported(what),
                (None, Some(what)) => reply::no_source(what),
                (None, None) => reply::failure(&error),
            })));
        }
    };
    let sender = standing.creator.clone();
    let (key, fate) = message::enqueue_keyed(daemon, &run, frame.content, sender).await;
    tokio::pin!(fate);
    let fate = tokio::select! {
        fate = &mut fate => fate,
        () = cancel(scope) => {
            message::cancel(&run, &key).await;
            fate.await
        }
    };
    Ok(match fate {
        message::Fate::Delivered => Some(Frame::Delivered),
        message::Fate::Cancelled => None,
        message::Fate::Error(error) => Some(Frame::Error(error)),
    })
}

/// The client's one channel, read for its cancel and never answered;
/// the inbox closing is not a cancel.
async fn cancel(scope: &ScopeHandle) {
    loop {
        let Some(bytes) = scope.recv_channel_request().await else {
            return std::future::pending().await;
        };
        if let Ok(ClientFrame::ChannelRequest { payload, .. }) = ClientFrame::decode(&bytes)
            && let Ok(channel_request::Frame::Cancel) = channel_request::Frame::decode(payload)
        {
            return;
        }
    }
}
