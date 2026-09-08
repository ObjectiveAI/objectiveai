//! What every handler shares.

use std::sync::Arc;

use axum::extract::FromRef;

use crate::agent_schema::AgentSchema;
use crate::filetree::Ignore;
use crate::requests::Requests;
use crate::run_loop::RunLoop;

/// The router's state: the ask table every exchange rides, the ignore
/// set the filetree honors, the meeting point of the loop's two sides,
/// and the schema the harness posted. A handler extracts the piece it
/// needs through [`FromRef`], so one that only asks never names the
/// others.
#[derive(Clone)]
pub struct AppState {
    pub requests: Arc<Requests>,
    pub ignore: Arc<Ignore>,
    pub run_loop: Arc<RunLoop>,
    pub agent_schema: Arc<AgentSchema>,
}

impl FromRef<AppState> for Arc<RunLoop> {
    fn from_ref(state: &AppState) -> Self {
        Arc::clone(&state.run_loop)
    }
}

impl FromRef<AppState> for Arc<AgentSchema> {
    fn from_ref(state: &AppState) -> Self {
        Arc::clone(&state.agent_schema)
    }
}

impl FromRef<AppState> for Arc<Requests> {
    fn from_ref(state: &AppState) -> Self {
        Arc::clone(&state.requests)
    }
}

impl FromRef<AppState> for Arc<Ignore> {
    fn from_ref(state: &AppState) -> Self {
        Arc::clone(&state.ignore)
    }
}
