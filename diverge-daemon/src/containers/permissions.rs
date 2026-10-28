//! What a tool's container declared it may ask of the daemon, kept on
//! its row once its run is up.

use diverge_sdk::shared::permission::Grant;

use crate::daemon::{Daemon, Kind};
use crate::judge::ToolRef;
use crate::store::{self, dependency_tools, tools};

/// Keep `permissions` — what the run's registration declared, empty
/// for a program that declared none — on the tool's row, as what it
/// last declared, and tell the tool's lists. Written at every start,
/// so the row follows the image.
pub async fn record(daemon: &Daemon, tool: ToolRef, permissions: &[Grant]) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    match tool {
        ToolRef::Standalone(id) => {
            tools::set_permissions(&mut conn, id, permissions).await?;
            daemon.live.changed(Kind::StandaloneTools);
        }
        ToolRef::Dependency(id) => {
            dependency_tools::set_permissions(&mut conn, id, permissions).await?;
            daemon.live.changed(Kind::DependencyTools);
        }
    }
    Ok(())
}
