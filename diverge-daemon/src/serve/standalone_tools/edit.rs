//! Editing a tool: a patch to its name, its account, its mounts —
//! while its container runs, to its FUSE mounts alone, and only by
//! addition.

use diverge_sdk::daemon::endpoints::standalone_tools::edit::client::request::{self, Editable};
use diverge_sdk::daemon::endpoints::standalone_tools::edit::server::response::Frame;
use diverge_sdk::daemon::grant::standalone_tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{reaches, resolve};
use crate::containers::ToolKey;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::inner::{self, Checked, Mounted, Outcome, Secrets, Which};
use crate::serve::reply;
use crate::store::tools::Origin;
use crate::store::{self, accounts, tools, tools_templates};

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
/// `NotOwned` for a result differing in anything but the name of a
/// connected tool, which has nothing else of this caller's to change;
/// `Active` for a result that, while the tool's container runs,
/// differs in anything but FUSE mounts added — the name, the account,
/// the volume mounts, the secrets, a FUSE mount removed, replaced or
/// reordered; `NoAccount`; the error for a provider that is not
/// there, or volumes named on a tool pinned to no provider; the
/// `secrets` error for secrets that are not the template's, and
/// `Forbidden` for a vault key the grants do not reach; `InUse` for a
/// name another tool has; else the tool as the result states, whole
/// or not at all.
///
/// The rules of an edit while the container runs are the agents
/// edit's, see [`agents::edit`](crate::serve::agents::edit): the run's
/// edit lock first and the record read under it; additions to the
/// FUSE lists the one change allowed; the additions checked as a
/// create's mounts are; the transaction let go and each addition
/// served and asked of the tool's provider, waited for; what the
/// provider mounted persisted, the first refusal or failure ending
/// the edit after it.
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
    // A connected tool runs nothing of this caller's: no mount is
    // added to it, and its name is edited as a record's is.
    let run = if tool.is_connected() { None } else { daemon.live.tool_run(ToolKey::Record(tool.id)).await };
    // The lock first, then the record again under it: an edit
    // persisted while the lock was waited for is what this one
    // patches.
    let locked = run.clone();
    let _edits = match &locked {
        Some(run) => Some(run.edits.lock().await),
        None => None,
    };
    let tool = match &run {
        Some(_) => match resolve(&mut tx, &frame.tool, true).await? {
            Some(tool) => tool,
            None => return Ok(Frame::NotFound),
        },
        None => tool,
    };
    if !reaches(&mut tx, daemon, &standing, Over::Edit, &tool).await? {
        return Ok(Frame::Forbidden);
    }
    let pinned = match &tool.origin {
        Origin::Created { provider, .. } => provider.clone(),
        Origin::Connected { .. } => None,
    };
    let account_identity = match tool.account {
        Some(id) => accounts::by_id(&mut tx, id).await?.map(|account| account.identity),
        None => None,
    };
    let current = Editable {
        name: tool.name.clone(),
        account: account_identity,
        volume_mounts: pinned.as_ref().map(|provider| provider.volume_mounts.clone()).unwrap_or_default(),
        fuse_file_mounts: tool.fuse_file_mounts.clone(),
        fuse_directory_mounts: tool.fuse_directory_mounts.clone(),
        secrets: tool.secrets.clone(),
    };
    let result = match inner::patched(&current, &frame.patch) {
        Ok(result) => result,
        Err(what) => return Ok(Frame::Error(reply::patch(&what))),
    };
    let mounts_changed = result.volume_mounts != current.volume_mounts
        || result.fuse_file_mounts != current.fuse_file_mounts
        || result.fuse_directory_mounts != current.fuse_directory_mounts;
    let account_changed = result.account != current.account;
    let secrets_changed = result.secrets != current.secrets;
    if tool.is_connected() && (mounts_changed || account_changed || secrets_changed) {
        return Ok(Frame::NotOwned);
    }
    let added = match &run {
        None => None,
        Some(_) => {
            if result.name != current.name || account_changed || result.volume_mounts != current.volume_mounts || secrets_changed {
                return Ok(Frame::Active);
            }
            match inner::additions(&current.fuse_file_mounts, &result.fuse_file_mounts, &current.fuse_directory_mounts, &result.fuse_directory_mounts) {
                Some(added) => Some(added),
                None => return Ok(Frame::Active),
            }
        }
    };
    let account = if account_changed {
        match inner::account(&mut tx, &standing, daemon, result.account.as_ref()).await? {
            Checked::Ok(account) => account,
            Checked::NoAccount => return Ok(Frame::NoAccount),
            Checked::Forbidden => return Ok(Frame::Forbidden),
            Checked::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
        }
    } else {
        tool.account
    };
    if secrets_changed {
        let Some(id) = tool.template() else {
            return Ok(Frame::NotOwned);
        };
        let Some(template) = tools_templates::by_id(&mut tx, id, false).await? else {
            return Ok(Frame::Error(reply::failure(&"the tool's template is not on record")));
        };
        match inner::secrets(&mut tx, &standing, &template.template.secrets, &result.secrets).await? {
            Secrets::Ok => {}
            Secrets::Mismatch(what) => return Ok(Frame::Error(reply::secrets(&what))),
            Secrets::Missing(what) => return Ok(Frame::Error(reply::secrets(&what))),
            Secrets::Forbidden => return Ok(Frame::Forbidden),
        }
    }
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
        match inner::providers(daemon, provider.as_ref(), &result.fuse_file_mounts, &result.fuse_directory_mounts) {
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
    let Some((run, (files, directories))) = run.zip(added) else {
        // Not running: the result is the tool, whole or not at all.
        let columns = tools::Columns {
            name: result.name,
            account,
            provider,
            fuse_file_mounts: result.fuse_file_mounts,
            fuse_directory_mounts: result.fuse_directory_mounts,
            secrets: result.secrets,
        };
        if let tools::Updated::InUse = tools::update(&mut tx, tool.id, &columns).await? {
            return Ok(Frame::InUse);
        }
        tx.commit().await?;
        daemon.live.changed(Kind::StandaloneTools);
        daemon.live.changed(Kind::Agents);
        daemon.live.changed(Kind::Volumes);
        daemon.live.changed(Kind::Vault);
        return Ok(Frame::Edited);
    };
    if files.is_empty() && directories.is_empty() {
        return Ok(Frame::Edited);
    }
    // Nothing is written yet; the provider is waited for with no
    // transaction open.
    drop(tx);
    let mut made_files = current.fuse_file_mounts;
    let mut made_directories = current.fuse_directory_mounts;
    let mut made = 0usize;
    let mut failure = None;
    for (mount, directory) in files.iter().map(|mount| (mount, false)).chain(directories.iter().map(|mount| (mount, true))) {
        match inner::runtime_mount(daemon, &run.mounts, Which::Tool(&run.handle), mount, directory).await {
            Outcome::Mounted => {
                made += 1;
                if directory {
                    made_directories.push(mount.clone());
                } else {
                    made_files.push(mount.clone());
                }
            }
            Outcome::Refused(refusal) => {
                failure = Some(Frame::Error(reply::mount(&refusal)));
                break;
            }
            Outcome::Failed(what) => {
                failure = Some(Frame::Error(reply::failure(&what)));
                break;
            }
        }
    }
    if made > 0 {
        let mut tx = daemon.store.begin().await?;
        let columns = tools::Columns {
            name: current.name,
            account,
            provider,
            fuse_file_mounts: made_files,
            fuse_directory_mounts: made_directories,
            secrets: current.secrets,
        };
        if let tools::Updated::InUse = tools::update(&mut tx, tool.id, &columns).await? {
            return Ok(Frame::InUse);
        }
        tx.commit().await?;
        daemon.live.changed(Kind::StandaloneTools);
        daemon.live.changed(Kind::Agents);
        daemon.live.changed(Kind::Volumes);
    }
    Ok(failure.unwrap_or(Frame::Edited))
}
