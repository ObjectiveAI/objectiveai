//! One container per tool on record: started for the first container
//! or exposure that uses it, stopped when nothing holds it.

use std::collections::HashSet;
use std::sync::Arc;

use super::{StartError, ToolKey, ToolRun, User, start};
use crate::daemon::Daemon;
use crate::store::tools::Tool;

/// The tool's run, with `user` among its users: started now if it was
/// not running, and touched if it was.
pub async fn use_tool(daemon: &Arc<Daemon>, tool: &Tool, user: User) -> Result<Arc<ToolRun>, StartError> {
    if let Some(run) = daemon.live.tool_run(ToolKey::Record(tool.id)).await {
        run.users.lock().await.insert(user);
        run.touch();
        return Ok(run);
    }
    let run = start::tool(daemon, tool).await?;
    run.users.lock().await.insert(user);
    Ok(run)
}

/// `user` is done with the tool: a record's run is stopped when it
/// was the last user and no connector is attached from outside. A
/// dependency is nobody's to release: it is stopped with its agent,
/// and this does nothing for one.
pub async fn release(daemon: &Daemon, tool: ToolKey, user: User) {
    if let ToolKey::Dependency(_) = tool {
        return;
    }
    let Some(run) = daemon.live.tool_run(tool).await else {
        return;
    };
    let mut users = run.users.lock().await;
    users.remove(&user);
    stop_if_unheld(&run, &users).await;
}

/// A connector left the tool: its run is stopped when none remains
/// and nothing of the daemon's uses it.
pub async fn disconnected(run: &ToolRun) {
    let users = run.users.lock().await;
    stop_if_unheld(run, &users).await;
}

/// Stop the run unless something holds it: the one test the last
/// user, the last exposure and the last connector leaving all come
/// to, under the users lock so that a user arriving between the test
/// and the stop is not lost.
async fn stop_if_unheld(run: &ToolRun, users: &HashSet<User>) {
    if !run.is_held(users) {
        run.handle.stop().await;
    }
}
