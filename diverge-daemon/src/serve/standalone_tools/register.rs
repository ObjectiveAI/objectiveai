//! Registering another daemon's tool under a name.

use diverge_sdk::daemon::endpoints::standalone_tools::register::client::request;
use diverge_sdk::daemon::endpoints::standalone_tools::register::server::response::Frame;
use diverge_sdk::daemon::grant::daemons::Action;
use diverge_sdk::daemon::grant::standalone_tools::Make;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use indexmap::IndexMap;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::tools::Origin;
use crate::store::{self, tools};

/// Answer the register and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` without the `register` grant, or without the `connect`
/// grant through the remote daemons; the error for a remote daemon the
/// configuration does not name; `InUse` for a name another tool has;
/// else the tool held — a connected one, with no account and no
/// mounts, its index the next among tools registered from that
/// daemon's tool — and nothing connected: the daemon is connected to
/// and its connect opened while an attached agent uses the tool.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::standalone_tools::make(&standing, Make::Register) {
        return Ok(Frame::Forbidden);
    }
    if !daemon.daemons.connect.iter().any(|remote| remote.name == frame.daemon) {
        return Ok(Frame::Error(reply::failure(&"the daemon named is none the configuration names")));
    }
    if !judge::daemons::holds(&standing, Action::Connect) {
        return Ok(Frame::Forbidden);
    }
    let new = tools::New {
        origin: Origin::Connected {
            daemon: frame.daemon,
            tool: frame.tool,
        },
        name: frame.name,
        account: None,
        fuse_file_mounts: Vec::new(),
        fuse_directory_mounts: Vec::new(),
        secrets: IndexMap::new(),
        creator: standing.creator.clone(),
    };
    match tools::create(&mut tx, &new).await? {
        tools::Created::Created(..) => {}
        tools::Created::Exists => return Ok(Frame::InUse),
    }
    tx.commit().await?;
    daemon.live.changed(Kind::StandaloneTools);
    Ok(Frame::Registered)
}
