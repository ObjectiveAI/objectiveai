//! What every handler shares.

use std::sync::Arc;

use axum::extract::FromRef;

use crate::agent::Upstream;
use crate::tool::Tool;
use crate::requests::Requests;

/// The router's state: the ask table every exchange rides, the client
/// the `/agent/*` paths dial the agent's server with, and the MCP
/// client the `/tool/*` paths call the tool's server through. A
/// handler extracts the piece it needs through [`FromRef`], so one
/// that only asks never names the others.
#[derive(Clone)]
pub struct AppState {
    pub requests: Arc<Requests>,
    pub upstream: Arc<Upstream>,
    pub tool: Arc<Tool>,
}

impl FromRef<AppState> for Arc<Tool> {
    fn from_ref(state: &AppState) -> Self {
        Arc::clone(&state.tool)
    }
}

impl FromRef<AppState> for Arc<Requests> {
    fn from_ref(state: &AppState) -> Self {
        Arc::clone(&state.requests)
    }
}

impl FromRef<AppState> for Arc<Upstream> {
    fn from_ref(state: &AppState) -> Self {
        Arc::clone(&state.upstream)
    }
}
