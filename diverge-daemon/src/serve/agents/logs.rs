//! Reading an agent's log, and watching it.

use chrono::Utc;
use diverge_sdk::daemon::endpoints::agents::logs::client::channel_request;
use diverge_sdk::daemon::endpoints::agents::logs::client::request;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Failure, active};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::logs;
use crate::serve::reply;
use crate::store::{AgentId, agents};

/// Send what the filter yields, watch if asked, and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `logs` grant at all, or for an agent the grants
/// do not reach; the error for an agent that is not there; else the
/// filter run over the log as it stands, oldest first, one frame per
/// item, and the finish — or, watching, over each item as it lands
/// until the count is met, the log reaches `logs_index_to`, an item
/// later than `created_to` lands or the clock passes it, the client
/// cancels, or the agent is deleted. Every item is sent once: the
/// historical read ends at an index, and the watch begins at the next.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), Failure> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::agents::holds(&standing, Over::Logs) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let Some(agent) = agents::by_reference(&mut conn, &frame.agent, false).await? else {
        reply::reply(scope, &Frame::Error(reply::failure(&"the agent named is none the daemon has"))).await;
        return Ok(());
    };
    drop(conn);
    if !judge::agents::over(&standing, Over::Logs, &agent, active(daemon, agent.id).await) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    // Subscribe before the first read, so an append between the read
    // and the wait is not missed; the handle itself is let go, so the
    // agent's deletion — the sender gone — ends the watch.
    let mut latest = daemon.live.log(agent.id).await.latest.subscribe();
    let watching = frame.watch.unwrap_or(false);
    let cap = frame.count.unwrap_or(u64::MAX);
    let mut sent = 0u64;
    let mut next = frame.logs_index_from.unwrap_or(1).max(1);
    loop {
        if read_through(scope, daemon, &frame, agent.id, &mut next, &mut sent, cap).await? {
            return Ok(());
        }
        if !watching {
            return Ok(());
        }
        let deadline = frame.created_to.and_then(|to| (to - Utc::now()).to_std().ok());
        tokio::select! {
            changed = latest.changed() => {
                if changed.is_err() {
                    return Ok(());
                }
            }
            cancelled = cancel(scope) => {
                if cancelled {
                    return Ok(());
                }
            }
            () = wait(deadline) => {
                return Ok(());
            }
        }
    }
}

/// Send every item from `next` through the log's end that the filter
/// yields, moving `next` past what was looked at; `true` when nothing
/// more can come — the count met, `logs_index_to` reached, or an item
/// later than `created_to` seen.
async fn read_through(
    scope: &ScopeHandle,
    daemon: &Daemon,
    frame: &request::Frame,
    id: AgentId,
    next: &mut u64,
    sent: &mut u64,
    cap: u64,
) -> Result<bool, Failure> {
    let count = logs::count(&daemon.logs, id).await?;
    let to = frame.logs_index_to.unwrap_or(u64::MAX).min(count);
    if *next <= to {
        for item in logs::read(&daemon.logs, id, *next, to).await? {
            if frame.created_to.is_some_and(|bound| item.created > bound) {
                return Ok(true);
            }
            if logs::matches(frame, &item) {
                reply::reply(scope, &Frame::Item(item)).await;
                *sent += 1;
                if *sent >= cap {
                    return Ok(true);
                }
            }
        }
        *next = to + 1;
    }
    Ok(frame.logs_index_to.is_some_and(|bound| count >= bound))
}

/// Whether the client cancelled: the one channel it may open, read
/// and never answered. The inbox closing is not a cancel — the finish
/// is the handler's.
async fn cancel(scope: &ScopeHandle) -> bool {
    loop {
        let Some(bytes) = scope.recv_channel_request().await else {
            return std::future::pending().await;
        };
        if let Ok(ClientFrame::ChannelRequest { payload, .. }) = ClientFrame::decode(&bytes)
            && let Ok(channel_request::Frame::Cancel) = channel_request::Frame::decode(payload)
        {
            return true;
        }
    }
}

/// The clock passing `created_to`, when there is one; never,
/// otherwise.
async fn wait(deadline: Option<std::time::Duration>) {
    match deadline {
        Some(deadline) => tokio::time::sleep(deadline).await,
        None => std::future::pending().await,
    }
}
