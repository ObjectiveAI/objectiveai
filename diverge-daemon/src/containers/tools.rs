//! One container per tool on record: started for the first container
//! or connect scope that uses it, stopped when nothing holds it.

use std::collections::HashSet;
use std::sync::Arc;

use super::{StartError, ToolHandle, ToolKey, ToolRun, User, start};
use crate::daemon::Daemon;
use crate::store::tools::Tool;

/// The tool's run, with `user` among its users: started now if it was
/// not running, and touched if it was.
pub async fn use_tool(daemon: &Arc<Daemon>, tool: &Tool, user: User) -> Result<Arc<ToolRun>, StartError> {
    if let Some(run) = daemon.live.tool_run(ToolKey::Record(tool.id)).await {
        let mut users = run.users.lock().await;
        users.insert(user);
        held(&run, &users);
        drop(users);
        run.touch();
        return Ok(run);
    }
    let run = start::tool(daemon, tool).await?;
    let mut users = run.users.lock().await;
    users.insert(user);
    held(&run, &users);
    drop(users);
    Ok(run)
}

/// `user` is done with the tool: a record's container is stopped when
/// it was the last user; a connected tool's connection is left to its
/// idle clock. A dependency is nobody's to release: it is stopped with
/// its agent, and this does nothing for one.
pub async fn release(daemon: &Daemon, tool: ToolKey, user: User) {
    if let ToolKey::Dependency(_) = tool {
        return;
    }
    let Some(run) = daemon.live.tool_run(tool).await else {
        return;
    };
    let mut users = run.users.lock().await;
    users.remove(&user);
    held(&run, &users);
    stop_if_unheld(&run, &users).await;
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
