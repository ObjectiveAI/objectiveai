//! Creating an agent.

use diverge_sdk::daemon::endpoints::agents::create::client::request;
use diverge_sdk::daemon::endpoints::agents::create::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::inner::{self, Checked, Mounted, Secrets};
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

/// `Forbidden` without the `create` grant, or for a vault key the
/// grants do not reach; the error for a template or a provider that
/// is not there; the `secrets` error for secrets that are not the
/// template's; `InUse` for a name another agent has; else the agent
/// made, its index the next for its template,
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
    let Some(template) = agents_templates::by_id(&mut tx, &inner.template, false).await? else {
        return Ok(Frame::Error(reply::failure(&"the agent template named is none the daemon has")));
    };
    match inner::secrets(&mut tx, &standing, &template.template.secrets, &inner.secrets).await? {
        Secrets::Ok => {}
        Secrets::Mismatch(what) => return Ok(Frame::Error(reply::secrets(&what))),
        Secrets::Missing(what) => return Ok(Frame::Error(reply::secrets(&what))),
        Secrets::Forbidden => return Ok(Frame::Forbidden),
    }
    match inner::providers(daemon, inner.provider.as_ref(), &inner.fuse_file_mounts, &inner.fuse_directory_mounts) {
        Checked::Ok(()) => {}
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
        secrets: inner.secrets,
        creator: standing.creator.clone(),
    };
    match agents::create(&mut tx, &new).await? {
        agents::Created::Created(..) => {}
        agents::Created::Exists => return Ok(Frame::InUse),
    }
    tx.commit().await?;
    daemon.live.changed(Kind::Agents);
    daemon.live.changed(Kind::Volumes);
    daemon.live.changed(Kind::Vault);
    Ok(Frame::Created)
}
