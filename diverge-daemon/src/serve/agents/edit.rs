//! Editing an agent: a patch to its name and its mounts.

use diverge_sdk::daemon::endpoints::agents::edit::client::request::{self, Editable};
use diverge_sdk::daemon::endpoints::agents::edit::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::active;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::inner::{self, Checked, Mounted};
use crate::serve::reply;
use crate::store::{self, agents};

/// Answer the edit and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `edit` grant at all; `NotFound`; `Forbidden`
/// for an agent the grants do not reach; the `patch` error for a
/// patch that could not be applied or a result that is not an agent's
/// editable members; `Active` for a result differing in any mount
/// list while a loop runs; the error for a provider that is not
/// there, or volumes named on an agent pinned to no provider; `InUse`
/// for a name another agent has; else the agent as the result states,
/// whole or not at all.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::agents::holds(&standing, Over::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(agent) = agents::by_reference(&mut tx, &frame.agent, true).await? else {
        return Ok(Frame::NotFound);
    };
    let active = active(daemon, agent.id).await;
    if !judge::agents::over(&standing, Over::Edit, &agent, active) {
        return Ok(Frame::Forbidden);
    }
    let current = Editable {
        name: agent.name.clone(),
        volume_mounts: agent.provider.as_ref().map(|provider| provider.volume_mounts.clone()).unwrap_or_default(),
        fuse_file_mounts: agent.fuse_file_mounts.clone(),
        fuse_directory_mounts: agent.fuse_directory_mounts.clone(),
    };
    let result = match inner::patched(&current, &frame.patch) {
        Ok(result) => result,
        Err(what) => return Ok(Frame::Error(reply::patch(&what))),
    };
    let mounts_changed = result.volume_mounts != current.volume_mounts
        || result.fuse_file_mounts != current.fuse_file_mounts
        || result.fuse_directory_mounts != current.fuse_directory_mounts;
    if mounts_changed && active {
        return Ok(Frame::Active);
    }
    let provider = match agent.provider.clone() {
        Some(mut provider) => {
            provider.volume_mounts = result.volume_mounts;
            Some(provider)
        }
        None => {
            if !result.volume_mounts.is_empty() {
                return Ok(Frame::Error(reply::failure(&"the agent is pinned to no provider, so no volume of one can be mounted")));
            }
            None
        }
    };
    if mounts_changed {
        match inner::providers(&mut tx, provider.as_ref(), &result.fuse_file_mounts, &result.fuse_directory_mounts).await? {
            Checked::Ok(()) => {}
            Checked::NoAccount | Checked::Forbidden => return Ok(Frame::Forbidden),
            Checked::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
        }
        match inner::mounts(&mut tx, &standing, daemon, provider.as_ref(), &result.fuse_file_mounts, &result.fuse_directory_mounts).await? {
            Mounted::Ok => {}
            Mounted::Forbidden => return Ok(Frame::Forbidden),
            Mounted::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
            Mounted::Unsupported(what) => return Ok(Frame::Error(reply::unsupported(&what))),
        }
    }
    let columns = agents::Columns {
        name: result.name,
        provider,
        fuse_file_mounts: result.fuse_file_mounts,
        fuse_directory_mounts: result.fuse_directory_mounts,
    };
    if let agents::Updated::InUse = agents::update(&mut tx, agent.id, &columns).await? {
        return Ok(Frame::InUse);
    }
    tx.commit().await?;
    daemon.live.changed(Kind::Agents);
    daemon.live.changed(Kind::StandaloneTools);
    daemon.live.changed(Kind::Volumes);
    Ok(Frame::Edited)
}
