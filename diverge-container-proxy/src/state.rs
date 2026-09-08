//! What every handler shares.

use std::sync::Arc;

use axum::extract::FromRef;

use crate::agent::Upstream;
use crate::filetree::Ignore;
use crate::requests::Requests;

/// The router's state: the ask table every exchange rides, the ignore
/// set the filetree honors, and the client the `/agent/*` paths dial
/// the agent's server with. A handler extracts the piece it needs
/// through [`FromRef`], so one that only asks never names the others.
#[derive(Clone)]
pub struct AppState {
    pub requests: Arc<Requests>,
    pub ignore: Arc<Ignore>,
    pub upstream: Arc<Upstream>,
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

impl FromRef<AppState> for Arc<Upstream> {
    fn from_ref(state: &AppState) -> Self {
        Arc::clone(&state.upstream)
    }
}
