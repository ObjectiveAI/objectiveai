//! Creating a role.

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::endpoints::roles::create::client::request;
use diverge_sdk::daemon::endpoints::roles::create::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, roles};

/// Answer the create and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` without the `create` grant; `Exists` for a name another
/// role has; else the role made, with its grants exactly as given —
/// the daemon reads them no further than decoding them.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::roles::create(&standing) {
        return Ok(Frame::Forbidden);
    }
    let new = roles::New {
        name: frame.name,
        description: frame.description,
        grants: frame.grants,
        creator: Creator::Client(Client {
            identity: standing.identity.clone(),
        }),
    };
    match roles::create(&mut tx, &new).await? {
        roles::Created::Created(_) => {}
        roles::Created::Exists => return Ok(Frame::Exists),
    }
    tx.commit().await?;
    daemon.live.changed(Kind::Roles);
    Ok(Frame::Created)
}
