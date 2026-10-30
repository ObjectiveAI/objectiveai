//! Reading a standalone tool's log, and watching it.

use diverge_sdk::daemon::endpoints::standalone_tools::logs::client::channel_request;
use diverge_sdk::daemon::endpoints::standalone_tools::logs::client::request;
use diverge_sdk::daemon::endpoints::standalone_tools::logs::server::response::{Frame, ItemWrapper};
use diverge_sdk::daemon::grant::standalone_tools::Over;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{reaches, resolve};
use crate::containers::ToolKey;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{logs, reply};

/// Why the read could not be answered: the store, or the log.
enum Failure {
    Store(crate::store::Error),
    Logs(crate::logs::Error),
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Store(error) => write!(f, "{error}"),
            Failure::Logs(error) => write!(f, "the log: {error}"),
        }
    }
}

impl From<crate::store::Error> for Failure {
    fn from(error: crate::store::Error) -> Self {
        Failure::Store(error)
    }
}

impl From<crate::logs::Error> for Failure {
    fn from(error: crate::logs::Error) -> Self {
        Failure::Logs(error)
    }
}

/// Send what the filter yields, watch if asked, and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `logs` grant at all, or for a tool the grants
/// do not reach; the error for a tool that is not there; else the log
/// sent and watched as [`logs`](crate::serve::logs) states.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), Failure> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::standalone_tools::holds(&standing, Over::Logs) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let Some(tool) = resolve(&mut conn, &frame.tool, false).await? else {
        reply::reply(scope, &Frame::Error(reply::failure(&"the tool named is none the daemon has"))).await;
        return Ok(());
    };
    if !reaches(&mut conn, daemon, &standing, Over::Logs, &tool).await? {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let key = ToolKey::Record(tool.id);
    drop(conn);
    // Subscribe before the first read, so an append between the read
    // and the wait is not missed; the handle itself is let go, so the
    // tool's deletion — the sender gone — ends the watch.
    let latest = daemon.live.tool_log(key).await.latest.subscribe();
    let span = logs::Span {
        from: frame.logs_index_from,
        to: frame.logs_index_to,
        created_to: frame.created_to,
        count: frame.count,
        watch: frame.watch,
    };
    let keep = |item: &ItemWrapper| crate::logs::tool_matches(frame.r#type, frame.created_from, frame.created_to, item);
    logs::send(scope, daemon, key, latest, &span, keep, Frame::Item, is_cancel).await?;
    Ok(())
}

/// Whether a channel request's payload is the cancel.
fn is_cancel(payload: &[u8]) -> bool {
    matches!(channel_request::Frame::decode(payload), Ok(channel_request::Frame::Cancel))
}
