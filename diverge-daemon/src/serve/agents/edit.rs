//! Editing an agent.

use diverge_sdk::daemon::edit::Change;
use diverge_sdk::daemon::endpoints::agents::edit::client::request;
use diverge_sdk::daemon::endpoints::agents::edit::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::active;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::inner::{self, Checked};
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
/// for an agent the grants do not reach, or an account named the
/// caller holds no `assign` over; `Active` for a change of any mount
/// list while a loop runs; `NoAccount`; the error for a provider or a
/// deployer that is not there, or volumes named on an agent pinned to
/// no provider; `InUse` for a name another agent has; else the agent
/// as the request states, whole or not at all.
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
    let edit = frame.edit;
    let mounts_named = edit.volume_mounts.is_some() || edit.fuse_file_mounts.is_some() || edit.fuse_directory_mounts.is_some();
    if mounts_named && active {
        return Ok(Frame::Active);
    }
    let name = match edit.name {
        None => agent.name.clone(),
        Some(Change::Delete) => None,
        Some(Change::Set(name)) => Some(name),
    };
    let account = match edit.account {
        None => agent.account,
        Some(Change::Delete) => None,
        Some(Change::Set(name)) => match inner::account(&mut tx, &standing, daemon, Some(&name)).await? {
            Checked::Ok(account) => account,
            Checked::NoAccount => return Ok(Frame::NoAccount),
            Checked::Forbidden => return Ok(Frame::Forbidden),
            Checked::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
        },
    };
    let provider = match edit.volume_mounts {
        None => agent.provider.clone(),
        Some(change) => match agent.provider.clone() {
            Some(mut provider) => {
                provider.volume_mounts = match change {
                    Change::Delete => Vec::new(),
                    Change::Set(volume_mounts) => volume_mounts,
                };
                Some(provider)
            }
            None => {
                if matches!(change, Change::Set(ref mounts) if !mounts.is_empty()) {
                    return Ok(Frame::Error(reply::failure(&"the agent is pinned to no provider, so no volume of one can be mounted")));
                }
                None
            }
        },
    };
    let fuse_file_mounts = match edit.fuse_file_mounts {
        None => agent.fuse_file_mounts.clone(),
        Some(Change::Delete) => Vec::new(),
        Some(Change::Set(mounts)) => mounts,
    };
    let fuse_directory_mounts = match edit.fuse_directory_mounts {
        None => agent.fuse_directory_mounts.clone(),
        Some(Change::Delete) => Vec::new(),
        Some(Change::Set(mounts)) => mounts,
    };
    if mounts_named {
        match inner::providers(&mut tx, provider.as_ref(), &fuse_file_mounts, &fuse_directory_mounts).await? {
            Checked::Ok(()) => {}
            Checked::NoAccount | Checked::Forbidden => return Ok(Frame::Forbidden),
            Checked::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
        }
        match inner::mounts(&mut tx, &standing, daemon, provider.as_ref(), &fuse_file_mounts, &fuse_directory_mounts).await? {
            Checked::Ok(()) => {}
            Checked::NoAccount | Checked::Forbidden => return Ok(Frame::Forbidden),
            Checked::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
        }
    }
    let deployer = match frame.deployer_agent {
        None => agent.deployer.clone(),
        Some(Change::Delete) => None,
        Some(Change::Set(reference)) => match inner::deployer(&mut tx, Some(&reference)).await? {
            Checked::Ok(deployer) => deployer,
            Checked::NoAccount | Checked::Forbidden => return Ok(Frame::Forbidden),
            Checked::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
        },
    };
    let columns = agents::Columns {
        name,
        account,
        provider,
        fuse_file_mounts,
        fuse_directory_mounts,
        deployer,
    };
    if let agents::Updated::InUse = agents::update(&mut tx, agent.id, &columns).await? {
        return Ok(Frame::InUse);
    }
    tx.commit().await?;
    daemon.live.changed(Kind::Agents);
    daemon.live.changed(Kind::Tools);
    daemon.live.changed(Kind::Volumes);
    Ok(Frame::Edited)
}
