//! A run ended because a mount of its lost its serve.

use std::sync::Arc;

use diverge_sdk::daemon::lifecycle::Reason;

use super::lifecycle::End;
use super::{AgentRun, ToolRun};

/// Wait for the agent's run to lose a mount's serve, and end it then.
/// The rules:
///
/// 1. **A lost serve ends the run.** The run's mounts say when the
///    serve of any volume mount ended while the run still held it —
///    the volume's provider gone, or the connection to the volume's
///    daemon gone, at any hop: see
///    [`Mounts::lost`](super::fuse::Mounts::lost). The container can no
///    longer read or write what it mounted, and nothing reopens a
///    serve under a running container, so the run is over.
/// 2. **As failed, with its reason.** The run is said to be ending as
///    unexpected, `mount_lost`, unless someone said how first; the gate
///    is marked `ending`, so nothing more is activated on the run or
///    sent to it, as for a batch that could not be fed; and the
///    container is told to stop. The pump hears the end and takes the
///    run down, its `undeployed` carrying the reason.
/// 3. **Nothing lost, nothing done.** A run that ends otherwise ends
///    this task with its others, and it does nothing.
pub async fn lost_agent(run: Arc<AgentRun>) {
    let mut lost = run.mounts.lost();
    if lost.wait_for(|lost| *lost).await.is_err() {
        return;
    }
    run.gate.lock().await.ending = true;
    run.end_as(End::Unexpected(Reason::MountLost)).await;
    let _ = run.handle.cancel().await;
}

/// Wait for the tool's run to lose a mount's serve, and end it then:
/// the rules of [`lost_agent`], for a standalone tool's container. Its
/// pump hears the end and takes the run down.
pub async fn lost_tool(run: Arc<ToolRun>) {
    let mut lost = run.mounts.lost();
    if lost.wait_for(|lost| *lost).await.is_err() {
        return;
    }
    run.end_as(End::Unexpected(Reason::MountLost)).await;
    run.handle.stop().await;
}
