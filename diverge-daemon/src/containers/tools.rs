//! One container per tool on record: started for the first container
//! that uses it, stopped with the last.

use std::sync::Arc;

use super::{Key, StartError, ToolKey, ToolRun, start};
use crate::daemon::Daemon;
use crate::store::tools::Tool;

/// The tool's run, with `user` among its users: started now if it was
/// not running.
pub async fn use_tool(daemon: &Arc<Daemon>, tool: &Tool, user: Key) -> Result<Arc<ToolRun>, StartError> {
    if let Some(run) = daemon.live.tool_run(ToolKey::Record(tool.id)).await {
        run.users.lock().await.insert(user);
        run.touch();
        return Ok(run);
    }
    let run = start::tool(daemon, tool).await?;
    run.users.lock().await.insert(user);
    Ok(run)
}

/// `user` is done with the tool: the last to leave stops a record's
/// run. A dependency is nobody's to release: it is stopped with its
/// agent, and this does nothing for one.
pub async fn release(daemon: &Daemon, tool: ToolKey, user: Key) {
    if let ToolKey::Dependency(_) = tool {
        return;
    }
    let Some(run) = daemon.live.tool_run(tool).await else {
        return;
    };
    let last = {
        let mut users = run.users.lock().await;
        users.remove(&user);
        users.is_empty()
    };
    if last {
        run.handle.stop().await;
    }
}
