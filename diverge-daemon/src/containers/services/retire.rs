//! A service retired: gone for good.

use std::sync::Arc;

use crate::containers::{Key, ToolKey, stop};
use crate::daemon::{Daemon, Kind};
use crate::store::{ServiceId, services};

/// Retire the service: its state taken out of the live set and its
/// supervisor stilled first, so that nothing starts it again; its
/// run, if one is up, stopped and taken down; its row deleted, its
/// attachments with it; its scope's password forgotten, since its key
/// is never seen again — the scope itself stays, to go with its
/// owner; and the clock stilled last, since the clock is what calls
/// this. A service already retired is nothing to do.
pub async fn retire(daemon: &Arc<Daemon>, id: ServiceId) {
    let state = daemon.live.remove_service_state(id).await;
    if let Some(state) = &state {
        if let Some(supervisor) = state.supervisor.lock().await.take() {
            supervisor.abort();
        }
    }
    if let Some(run) = daemon.live.tool_run(ToolKey::Service(id)).await {
        run.handle.stop().await;
        run.handle.wait().await;
        stop::ended_tool(daemon, run).await;
    }
    if let Ok(mut conn) = daemon.store.acquire().await {
        let _ = services::delete(&mut conn, id).await;
    }
    daemon.live.forget_scope(Key::Tool(ToolKey::Service(id))).await;
    daemon.live.changed(Kind::Tools);
    if let Some(state) = state {
        if let Some(clock) = state.clock.lock().await.take() {
            clock.abort();
        }
    }
}
