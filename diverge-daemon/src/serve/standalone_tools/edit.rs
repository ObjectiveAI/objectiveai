//! Editing a tool: a patch to its name, its account, its mounts.

use diverge_sdk::daemon::endpoints::standalone_tools::edit::client::request::{self, Editable};
use diverge_sdk::daemon::endpoints::standalone_tools::edit::server::response::Frame;
use diverge_sdk::daemon::grant::standalone_tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{active, reaches, resolve};
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::inner::{self, Checked, Mounted};
use crate::serve::reply;
use crate::store::tools::Origin;
use crate::store::{self, accounts, tools};

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
/// holds no `assign` over; the `patch` error for a patch that could
/// not be applied or a result that is not a tool's editable members;
/// `Active` for a result differing in any mount list while the tool
/// is active; `NotOwned` for a result differing in anything but the
/// name of a connected tool, which has nothing else of this caller's
/// to change; `NoAccount`; the error for a provider that is not
/// there, or volumes named on a tool pinned to no provider; `InUse`
/// for a name another tool has; else the tool as the result states,
/// whole or not at all.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::standalone_tools::holds(&standing, Over::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = resolve(&mut tx, &frame.tool, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !reaches(&mut tx, daemon, &standing, Over::Edit, &tool).await? {
        return Ok(Frame::Forbidden);
    }
    let active = active(daemon, tool.id).await;
    let pinned = match &tool.origin {
        Origin::Created { provider, .. } => provider.clone(),
        Origin::Connected { .. } => None,
    };
    // An account named on a tool has a name: taking the name from one
    // a container runs under is refused.
    let account_name = match tool.account {
        Some(id) => accounts::by_id(&mut tx, id).await?.and_then(|account| account.name),
        None => None,
    };
    let current = Editable {
        name: tool.name.clone(),
        account: account_name,
        volume_mounts: pinned.as_ref().map(|provider| provider.volume_mounts.clone()).unwrap_or_default(),
        fuse_file_mounts: tool.fuse_file_mounts.clone(),
        fuse_directory_mounts: tool.fuse_directory_mounts.clone(),
    };
    let result = match inner::patched(&current, &frame.patch) {
        Ok(result) => result,
        Err(what) => return Ok(Frame::Error(reply::patch(&what))),
    };
    let mounts_changed = result.volume_mounts != current.volume_mounts
        || result.fuse_file_mounts != current.fuse_file_mounts
        || result.fuse_directory_mounts != current.fuse_directory_mounts;
    let account_changed = result.account != current.account;
    if mounts_changed && active {
        return Ok(Frame::Active);
    }
    if tool.is_connected() && (mounts_changed || account_changed) {
        return Ok(Frame::NotOwned);
    }
    let account = if account_changed {
        match inner::account(&mut tx, &standing, daemon, result.account.as_deref()).await? {
            Checked::Ok(account) => account,
            Checked::NoAccount => return Ok(Frame::NoAccount),
            Checked::Forbidden => return Ok(Frame::Forbidden),
            Checked::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
        }
    } else {
        tool.account
    };
    let provider = match pinned {
        Some(mut provider) => {
            provider.volume_mounts = result.volume_mounts;
            Some(provider)
        }
        None => {
            if !result.volume_mounts.is_empty() {
                return Ok(Frame::Error(reply::failure(&"the tool is pinned to no provider, so no volume of one can be mounted")));
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
    let columns = tools::Columns {
        name: result.name,
        account,
        provider,
        fuse_file_mounts: result.fuse_file_mounts,
        fuse_directory_mounts: result.fuse_directory_mounts,
    };
    if let tools::Updated::InUse = tools::update(&mut tx, tool.id, &columns).await? {
        return Ok(Frame::InUse);
    }
    tx.commit().await?;
    daemon.live.changed(Kind::StandaloneTools);
    daemon.live.changed(Kind::Agents);
    daemon.live.changed(Kind::Volumes);
    Ok(Frame::Edited)
}
