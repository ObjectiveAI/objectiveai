//! Holding another daemon's tool under a name.

use diverge_sdk::daemon::endpoints::connected_standalone_tools::create::client::request;
use diverge_sdk::daemon::endpoints::connected_standalone_tools::create::server::response::Frame;
use diverge_sdk::daemon::grant::connected_standalone_tools::Make;
use diverge_sdk::daemon::grant::daemons::Over as DaemonsOver;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, connected_standalone_tools};

/// Answer the create and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// The rules:
///
/// 1. No standing, or no `create` grant over connected tools, is
///    `Forbidden`.
/// 2. A `daemon` no record of this daemon's has is the error.
/// 3. A record the grants' `connect` over daemons does not reach is
///    `Forbidden`.
/// 4. A name a standalone or a connected tool has is `InUse`, as
///    [`tool_names`](crate::store::tool_names) states.
/// 5. Else the tool held — its index the next among the connected
///    tools held from that daemon's tool — and nothing connected: the
///    daemon is connected to and its connect opened while an attached
///    agent uses the tool.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::connected_standalone_tools::make(&standing, Make::Create) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = store::daemons::by_id(&mut tx, frame.daemon, false).await? else {
        return Ok(Frame::Error(reply::failure(&format!("no remote daemon's record has the ID `{}`", frame.daemon))));
    };
    let reported = record.report(daemon.live.is_daemon_connected(record.id).await);
    if !judge::daemons::over(&standing, DaemonsOver::Connect, &reported) {
        return Ok(Frame::Forbidden);
    }
    let new = connected_standalone_tools::New {
        daemon: frame.daemon,
        route: frame.route,
        tool: frame.tool,
        name: frame.name,
        creator: standing.creator.clone(),
    };
    match connected_standalone_tools::create(&mut tx, &new).await? {
        connected_standalone_tools::Created::Created(..) => {}
        connected_standalone_tools::Created::Exists => return Ok(Frame::InUse),
    }
    tx.commit().await?;
    daemon.live.changed(Kind::ConnectedStandaloneTools);
    Ok(Frame::Created)
}
