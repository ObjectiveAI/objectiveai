//! Getting one remote daemon.

use diverge_sdk::daemon::endpoints::providers::daemons::get::client::request;
use diverge_sdk::daemon::endpoints::providers::daemons::get::server::response::Frame;
use diverge_sdk::daemon::endpoints::providers::daemons::list::server::response;
use diverge_sdk::daemon::grant::daemons::Action;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::config::Remote;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;

/// Answer the get and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `get` grant; `NotFound` for a name the
/// configuration does not give; else the remote as a list reports it.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::daemons::holds(&standing, Action::Get) {
        return Ok(Frame::Forbidden);
    }
    let Some(remote) = daemon.daemons.connect.iter().find(|remote| remote.name == frame.name) else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_daemon_connected(&remote.name).await;
    Ok(Frame::Found(report(remote, connected)))
}

/// The remote as a get and a list report it, `connected` or not.
pub fn report(remote: &Remote, connected: bool) -> response::Daemon {
    response::Daemon {
        name: remote.name.clone(),
        issuer: remote.issuer.clone(),
        links: remote.links.clone(),
        connected,
    }
}
