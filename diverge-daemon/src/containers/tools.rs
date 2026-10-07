//! One container per tool: started for the first container that uses
//! it, stopped with the last.

use std::sync::Arc;

use super::{Key, StartError, ToolRun, start};
use crate::daemon::Daemon;
use crate::store::ToolId;
use crate::store::tools::Tool;

/// The tool's run, with `user` among its users: started now if it was
/// not running, on the chain `user`'s run is on.
pub async fn use_tool(daemon: &Arc<Daemon>, tool: &Tool, user: Key, root: Option<String>, chain: Vec<String>) -> Result<Arc<ToolRun>, StartError> {
    if let Some(run) = daemon.live.tool_run(tool.id).await {
        run.users.lock().await.insert(user);
        run.touch();
        return Ok(run);
    }
    let run = start::tool(daemon, tool, root, chain).await?;
    run.users.lock().await.insert(user);
    Ok(run)
}

/// `user` is done with the tool: the last to leave stops its run.
pub async fn release(daemon: &Daemon, tool: ToolId, user: Key) {
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
