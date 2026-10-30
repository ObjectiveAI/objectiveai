//! Reading an agent's log, and watching it.

use diverge_sdk::daemon::endpoints::agents::logs::client::channel_request;
use diverge_sdk::daemon::endpoints::agents::logs::client::request;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::{Frame, ItemWrapper};
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Failure, active};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{logs, reply};
use crate::store::agents;

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
    let latest = daemon.live.log(agent.id).await.latest.subscribe();
    let span = logs::Span {
        from: frame.logs_index_from,
        to: frame.logs_index_to,
        created_to: frame.created_to,
        count: frame.count,
        watch: frame.watch,
    };
    let keep = |item: &ItemWrapper| crate::logs::matches(&frame, item);
    logs::send(scope, daemon, agent.id, latest, &span, keep, Frame::Item, is_cancel).await?;
    Ok(())
}

/// Whether a channel request's payload is the cancel.
fn is_cancel(payload: &[u8]) -> bool {
    matches!(channel_request::Frame::decode(payload), Ok(channel_request::Frame::Cancel))
}
