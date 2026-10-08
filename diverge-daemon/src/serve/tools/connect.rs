//! Holding somebody else's tool container under a name.

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::endpoints::tools::connect::client::request;
use diverge_sdk::daemon::endpoints::tools::connect::server::response::Frame;
use diverge_sdk::daemon::grant::tools::Make;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::inner;
use crate::serve::reply;
use crate::store::tools::Origin;
use crate::store::{self, tools};

/// Answer the connect and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` without the `connect` grant; the error for a provider
/// the daemon does not know; `InUse` for a name another tool has;
/// else the tool held — a connected one, with no account, no mounts
/// and no deployer, its index the next among tools joined to that
/// container — and nothing joined: the connect scope is held while an
/// attached agent is active.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::tools::make(&standing, Make::Connect) {
        return Ok(Frame::Forbidden);
    }
    if !inner::on_record(&mut tx, &frame.provider).await? {
        return Ok(Frame::Error(reply::failure(&"the provider named is none the daemon knows")));
    }
    let new = tools::New {
        origin: Origin::Connected {
            provider: frame.provider,
            id: frame.id,
            authorization: frame.authorization,
        },
        name: frame.name,
        account: None,
        fuse_file_mounts: Vec::new(),
        fuse_directory_mounts: Vec::new(),
        deployer: None,
        creator: Creator::Client(Client {
            identity: standing.identity.clone(),
        }),
    };
    match tools::create(&mut tx, &new).await? {
        tools::Created::Created(..) => {}
        tools::Created::Exists => return Ok(Frame::InUse),
    }
    tx.commit().await?;
    daemon.live.changed(Kind::Tools);
    Ok(Frame::Connected)
}
