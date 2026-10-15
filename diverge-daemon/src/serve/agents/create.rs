//! Creating an agent.

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::endpoints::agents::create::client::request;
use diverge_sdk::daemon::endpoints::agents::create::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::inner::{self, Checked, Mounted};
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

/// `Forbidden` without the `create` grant; the error for a template
/// or a provider that is not there; `InUse` for a name another agent
/// has; else the agent made, its index the next for its template,
/// nothing running: the container is work made from the record on
/// its first use. No account: an agent holds none.
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
    match inner::providers(&mut tx, inner.provider.as_ref(), &inner.fuse_file_mounts, &inner.fuse_directory_mounts).await? {
        Checked::Ok(()) => {}
        Checked::NoAccount | Checked::Forbidden => return Ok(Frame::Forbidden),
        Checked::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
    }
    match inner::mounts(&mut tx, &standing, daemon, inner.provider.as_ref(), &inner.fuse_file_mounts, &inner.fuse_directory_mounts).await? {
        Mounted::Ok => {}
        Mounted::Forbidden => return Ok(Frame::Forbidden),
        Mounted::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
        Mounted::Unsupported(what) => return Ok(Frame::Error(reply::unsupported(&what))),
    }
    let new = agents::New {
        template: inner.template,
        name: frame.name,
        provider: inner.provider,
        fuse_file_mounts: inner.fuse_file_mounts,
        fuse_directory_mounts: inner.fuse_directory_mounts,
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
