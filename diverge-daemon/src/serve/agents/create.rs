//! Creating an agent.

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::endpoints::agents::create::client::request;
use diverge_sdk::daemon::endpoints::agents::create::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::inner::{self, Checked};
use crate::serve::reply;
use crate::store::{self, agents, agents_templates};

/// Answer the create and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` without the `create` grant, or without `assign` over
/// the account named; the error for a template, a provider or a
/// deployer that is not there; `NoAccount`; `InUse` for a name another
/// agent has; else the agent made, its index the next for its
/// template, nothing running: the container is work made from the
/// record on its first use.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::agents::create(&standing) {
        return Ok(Frame::Forbidden);
    }
    let inner = frame.inner;
    if agents_templates::by_id(&mut tx, &inner.template, false).await?.is_none() {
        return Ok(Frame::Error(reply::failure(&"the agent template named is none the daemon has")));
    }
    let account = match inner::account(&mut tx, &standing, daemon, inner.account.as_deref()).await? {
        Checked::Ok(account) => account,
        Checked::NoAccount => return Ok(Frame::NoAccount),
        Checked::Forbidden => return Ok(Frame::Forbidden),
        Checked::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
    };
    match inner::providers(&mut tx, inner.provider.as_ref(), &inner.fuse_file_mounts, &inner.fuse_directory_mounts).await? {
        Checked::Ok(()) => {}
        Checked::NoAccount | Checked::Forbidden => return Ok(Frame::Forbidden),
        Checked::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
    }
    match inner::mounts(&mut tx, &standing, daemon, inner.provider.as_ref(), &inner.fuse_file_mounts, &inner.fuse_directory_mounts).await? {
        Checked::Ok(()) => {}
        Checked::NoAccount | Checked::Forbidden => return Ok(Frame::Forbidden),
        Checked::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
    }
    let deployer = match inner::deployer(&mut tx, inner.deployer_agent.as_ref()).await? {
        Checked::Ok(deployer) => deployer,
        Checked::NoAccount | Checked::Forbidden => return Ok(Frame::Forbidden),
        Checked::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
    };
    let new = agents::New {
        template: inner.template,
        name: frame.name,
        account,
        provider: inner.provider,
        fuse_file_mounts: inner.fuse_file_mounts,
        fuse_directory_mounts: inner.fuse_directory_mounts,
        deployer,
        creator: Creator::Client(Client {
            identity: standing.identity.clone(),
        }),
    };
    match agents::create(&mut tx, &new).await? {
        agents::Created::Created(..) => {}
        agents::Created::Exists => return Ok(Frame::InUse),
    }
    tx.commit().await?;
    daemon.live.changed(Kind::Agents);
    daemon.live.changed(Kind::Volumes);
    Ok(Frame::Created)
}
