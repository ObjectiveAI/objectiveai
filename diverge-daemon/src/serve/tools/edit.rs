//! Editing a tool.

use diverge_sdk::daemon::edit::Change;
use diverge_sdk::daemon::endpoints::tools::edit::client::request;
use diverge_sdk::daemon::endpoints::tools::edit::server::response::Frame;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{active, agents_of};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::inner::{self, Checked};
use crate::serve::reply;
use crate::store::tools::Origin;
use crate::store::{self, tools};

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
/// for a tool the grants do not reach, or an account named the caller
/// holds no `assign` over; `Active` for a change of any mount list
/// while the tool is active; `NotOwned` for mounts or an account
/// named on a connected tool, which has neither to change;
/// `NoAccount`; the error for a provider or a deployer that is not
/// there, or volumes named on a tool pinned to no provider; `InUse`
/// for a name another tool has; else the tool as the request states,
/// whole or not at all.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::tools::holds(&standing, Over::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = tools::by_reference(&mut tx, &frame.tool, true).await? else {
        return Ok(Frame::NotFound);
    };
    let active = active(daemon, tool.id).await;
    let attached = agents_of(&mut tx, tool.id).await?;
    if !judge::tools::over(&standing, Over::Edit, &tool, active, &attached) {
        return Ok(Frame::Forbidden);
    }
    let edit = frame.edit;
    let mounts_named = edit.volume_mounts.is_some() || edit.fuse_file_mounts.is_some() || edit.fuse_directory_mounts.is_some();
    if mounts_named && active {
        return Ok(Frame::Active);
    }
    if tool.is_connected() && (mounts_named || edit.account.is_some()) {
        return Ok(Frame::NotOwned);
    }
    let name = match edit.name {
        None => tool.name.clone(),
        Some(Change::Delete) => None,
        Some(Change::Set(name)) => Some(name),
    };
    let account = match edit.account {
        None => tool.account,
        Some(Change::Delete) => None,
        Some(Change::Set(name)) => match inner::account(&mut tx, &standing, daemon, Some(&name)).await? {
            Checked::Ok(account) => account,
            Checked::NoAccount => return Ok(Frame::NoAccount),
            Checked::Forbidden => return Ok(Frame::Forbidden),
            Checked::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
        },
    };
    let pinned = match &tool.origin {
        Origin::Created { provider, .. } => provider.clone(),
        Origin::Connected { .. } => None,
    };
    let provider = match edit.volume_mounts {
        None => pinned,
        Some(change) => match pinned {
            Some(mut provider) => {
                provider.volume_mounts = match change {
                    Change::Delete => Vec::new(),
                    Change::Set(volume_mounts) => volume_mounts,
                };
                Some(provider)
            }
            None => {
                if matches!(change, Change::Set(ref mounts) if !mounts.is_empty()) {
                    return Ok(Frame::Error(reply::failure(&"the tool is pinned to no provider, so no volume of one can be mounted")));
                }
                None
            }
        },
    };
    let fuse_file_mounts = match edit.fuse_file_mounts {
        None => tool.fuse_file_mounts.clone(),
        Some(Change::Delete) => Vec::new(),
        Some(Change::Set(mounts)) => mounts,
    };
    let fuse_directory_mounts = match edit.fuse_directory_mounts {
        None => tool.fuse_directory_mounts.clone(),
        Some(Change::Delete) => Vec::new(),
        Some(Change::Set(mounts)) => mounts,
    };
    if mounts_named {
        match inner::providers(&mut tx, provider.as_ref(), &fuse_file_mounts, &fuse_directory_mounts).await? {
            Checked::Ok(()) => {}
            Checked::NoAccount | Checked::Forbidden => return Ok(Frame::Forbidden),
            Checked::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
        }
    }
    let deployer = match edit.deployer_agent {
        None => tool.deployer.clone(),
        Some(Change::Delete) => None,
        Some(Change::Set(reference)) => match inner::deployer(&mut tx, Some(&reference)).await? {
            Checked::Ok(deployer) => deployer,
            Checked::NoAccount | Checked::Forbidden => return Ok(Frame::Forbidden),
            Checked::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
        },
    };
    let columns = tools::Columns {
        name,
        account,
        provider,
        fuse_file_mounts,
        fuse_directory_mounts,
        deployer,
    };
    if let tools::Updated::InUse = tools::update(&mut tx, tool.id, &columns).await? {
        return Ok(Frame::InUse);
    }
    tx.commit().await?;
    Ok(Frame::Edited)
}
