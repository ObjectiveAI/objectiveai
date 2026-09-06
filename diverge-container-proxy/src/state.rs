//! What every handler shares.

use std::sync::Arc;

use axum::extract::FromRef;

use crate::filetree::Ignore;
use crate::requests::Requests;

/// The router's state: the ask table every exchange rides, and the
/// ignore set the filetree honors. A handler extracts the piece it
/// needs through [`FromRef`], so one that only asks never names the
/// other.
#[derive(Clone)]
pub struct AppState {
    pub requests: Arc<Requests>,
    pub ignore: Arc<Ignore>,
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
