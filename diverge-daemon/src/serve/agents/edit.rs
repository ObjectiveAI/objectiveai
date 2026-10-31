//! Editing an agent: a patch to its name and its mounts — while its
//! container runs, to its FUSE mounts alone, and only by addition.

use diverge_sdk::daemon::endpoints::agents::edit::client::request::{self, Editable};
use diverge_sdk::daemon::endpoints::agents::edit::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::state;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::inner::{self, Checked, Mounted, Outcome, Secrets, Which};
use crate::serve::reply;
use crate::store::{self, agents, agents_templates};

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
/// editable members; `Active` for a result that, while the agent's
/// container runs, differs in anything but FUSE mounts added — the
/// name, the volume mounts, the secrets, a FUSE mount removed,
/// replaced or reordered; the error for a provider that is not
/// there, or volumes named on an agent pinned to no provider; the
/// `secrets` error for secrets that are not the template's, and
/// `Forbidden` for a vault key the grants do not reach; `InUse` for a
/// name another agent has; else the agent as the result states,
/// whole or not at all.
///
/// The rules of an edit while the container runs:
///
/// 1. The run's edit lock is taken before the record is read under
///    it, and held to the end: two edits of one running agent are
///    made one after the other, the second patching what the first
///    persisted.
/// 2. The result may differ from the record only by FUSE mounts
///    appended to either list; anything else is `Active`, nothing
///    changed. No addition is `Edited`, nothing done.
/// 3. The additions are checked as a create's mounts are — the
///    providers on record, the volumes listed, the grants, the
///    capabilities — before anything is opened.
/// 4. The transaction is let go with nothing written, and each
///    addition in order — the files, then the directories — is served
///    and asked of the agent's provider, which is waited for: see
///    [`runtime_mount`](inner::runtime_mount).
/// 5. What the provider mounted is what persists: the record's lists
///    with every addition made, written in a transaction of their
///    own. The first addition refused or failed ends the edit with
///    the `mount` error, or the daemon's own, after what was made
///    before it is persisted; every addition made is `Edited`.
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
    let run = daemon.live.agent_run(agent.id).await;
    // The lock first, then the record again under it: an edit
    // persisted while the lock was waited for is what this one
    // patches.
    let locked = run.clone();
    let _edits = match &locked {
        Some(run) => Some(run.edits.lock().await),
        None => None,
    };
    let agent = match &run {
        Some(_) => match agents::by_reference(&mut tx, &frame.agent, true).await? {
            Some(agent) => agent,
            None => return Ok(Frame::NotFound),
        },
        None => agent,
    };
    let state = state(daemon, agent.id).await;
    if !judge::agents::over(&standing, Over::Edit, &agent, state) {
        return Ok(Frame::Forbidden);
    }
    let current = Editable {
        name: agent.name.clone(),
        volume_mounts: agent.provider.as_ref().map(|provider| provider.volume_mounts.clone()).unwrap_or_default(),
        fuse_file_mounts: agent.fuse_file_mounts.clone(),
        fuse_directory_mounts: agent.fuse_directory_mounts.clone(),
        secrets: agent.secrets.clone(),
    };
    let result = match inner::patched(&current, &frame.patch) {
        Ok(result) => result,
        Err(what) => return Ok(Frame::Error(reply::patch(&what))),
    };
    let mounts_changed = result.volume_mounts != current.volume_mounts
        || result.fuse_file_mounts != current.fuse_file_mounts
        || result.fuse_directory_mounts != current.fuse_directory_mounts;
    let added = match &run {
        None => None,
        Some(_) => {
            if result.name != current.name || result.volume_mounts != current.volume_mounts || result.secrets != current.secrets {
                return Ok(Frame::Active);
            }
            match inner::additions(&current.fuse_file_mounts, &result.fuse_file_mounts, &current.fuse_directory_mounts, &result.fuse_directory_mounts) {
                Some(added) => Some(added),
                None => return Ok(Frame::Active),
            }
        }
    };
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
    if result.secrets != current.secrets {
        let Some(template) = agents_templates::by_id(&mut tx, &agent.template, false).await? else {
            return Ok(Frame::Error(reply::failure(&"the agent's template is not on record")));
        };
        match inner::secrets(&mut tx, &standing, &template.template.secrets, &result.secrets).await? {
            Secrets::Ok => {}
            Secrets::Mismatch(what) => return Ok(Frame::Error(reply::secrets(&what))),
            Secrets::Missing(what) => return Ok(Frame::Error(reply::secrets(&what))),
            Secrets::Forbidden => return Ok(Frame::Forbidden),
        }
    }
    if mounts_changed {
        match inner::providers(daemon, provider.as_ref(), &result.fuse_file_mounts, &result.fuse_directory_mounts) {
            Checked::Ok(()) => {}
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
        // Not running: the result is the agent, whole or not at all.
        let columns = agents::Columns {
            name: result.name,
            provider,
            fuse_file_mounts: result.fuse_file_mounts,
            fuse_directory_mounts: result.fuse_directory_mounts,
            secrets: result.secrets,
        };
        if let agents::Updated::InUse = agents::update(&mut tx, agent.id, &columns).await? {
            return Ok(Frame::InUse);
        }
        tx.commit().await?;
        daemon.live.changed(Kind::Agents);
        daemon.live.changed(Kind::StandaloneTools);
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
        match inner::runtime_mount(daemon, &run.mounts, Which::Agent(&run.handle), mount, directory).await {
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
        let columns = agents::Columns {
            name: current.name,
            provider,
            fuse_file_mounts: made_files,
            fuse_directory_mounts: made_directories,
            secrets: current.secrets,
        };
        if let agents::Updated::InUse = agents::update(&mut tx, agent.id, &columns).await? {
            return Ok(Frame::InUse);
        }
        tx.commit().await?;
        daemon.live.changed(Kind::Agents);
        daemon.live.changed(Kind::StandaloneTools);
        daemon.live.changed(Kind::Volumes);
    }
    Ok(failure.unwrap_or(Frame::Edited))
}
