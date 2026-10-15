//! A service kept: its state made, its supervisor and its clock
//! started.

use std::sync::Arc;

use super::{State, clock, supervise};
use crate::containers::ToolKey;
use crate::daemon::Daemon;

/// Keep the service: its [`State`] made if it had none, held as of
/// now, and its supervisor and its clock started on tasks of their
/// own. One kept already is left as it is. Two keeps racing make one
/// state: the first in wins, and the second starts nothing.
pub async fn keep(daemon: &Arc<Daemon>, key: ToolKey) {
    let (state, made) = daemon.live.keep_service_state(key, Arc::new(State::new())).await;
    if !made {
        return;
    }
    let supervisor = tokio::spawn(supervise(Arc::clone(daemon), key, Arc::clone(&state)));
    let clock = tokio::spawn(clock(Arc::clone(daemon), key, Arc::clone(&state)));
    *state.supervisor.lock().await = Some(supervisor.abort_handle());
    *state.clock.lock().await = Some(clock.abort_handle());
}
