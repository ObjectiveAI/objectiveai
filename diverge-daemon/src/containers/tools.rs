//! One container per tool on record: started for the first container
//! or connect scope that uses it, stopped when nothing holds it — or,
//! a service, kept while anything holds it and retired by its clock.

use std::collections::HashSet;
use std::sync::Arc;

use super::{StartError, ToolHandle, ToolKey, ToolRun, User, services, start};
use crate::daemon::Daemon;
use crate::store::tools::Tool;

/// The tool's run, with `user` among its users: started now if it was
/// not running, and touched if it was. A service's run is kept from
/// here on — its supervisor and its clock started if they were not —
/// and told what holds it, since a connect scope among the users is
/// a hold.
pub async fn use_tool(daemon: &Arc<Daemon>, tool: &Tool, user: User) -> Result<Arc<ToolRun>, StartError> {
    let run = match daemon.live.tool_run(ToolKey::Record(tool.id)).await {
        Some(run) => {
            run.touch();
            run
        }
        None => start::tool(daemon, tool).await?,
    };
    {
        let mut users = run.users.lock().await;
        users.insert(user);
        held(&run, &users);
    }
    if run.service {
        services::keep(daemon, run.id).await;
        services::held(daemon, run.id).await;
    }
    Ok(run)
}

/// `user` is done with the tool: a record's container is stopped when
/// it was the last user, unless the tool is a service, which is held
/// by its attachments and its connect scopes and retired by its clock
/// — it is told what holds it instead; a connected tool's connection
/// is left to its idle clock. A dependency tool is nobody's to
/// release: a transient one is stopped with its agent's run and a
/// service is held by its attachments, and this does nothing for
/// either.
pub async fn release(daemon: &Daemon, tool: ToolKey, user: User) {
    if let ToolKey::Dependency(_) = tool {
        return;
    }
    let Some(run) = daemon.live.tool_run(tool).await else {
        return;
    };
    {
        let mut users = run.users.lock().await;
        users.remove(&user);
        held(&run, &users);
        if !run.service {
            stop_if_unheld(&run, &users).await;
        }
    }
    if run.service {
        services::held(daemon, run.id).await;
    }
}

/// The word that something uses the run, or nothing does, kept in
/// step with `users` under its lock.
fn held(run: &ToolRun, users: &HashSet<User>) {
    run.held.send_replace(run.is_held(users));
}

/// Stop a container the daemon runs unless something holds it: the one
/// test the last user and the last connect scope leaving both come
/// to, under the users lock so that a user arriving between the test
/// and the stop is not lost. A connected tool is not stopped here: its
/// connection is let go by its idle clock, `idle_seconds` after the
/// last use.
async fn stop_if_unheld(run: &ToolRun, users: &HashSet<User>) {
    if let ToolHandle::Run(_) = run.handle
        && !run.is_held(users)
    {
        run.handle.stop().await;
    }
}
