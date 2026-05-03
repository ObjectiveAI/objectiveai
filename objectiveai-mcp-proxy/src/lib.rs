//! ObjectiveAI MCP proxy library.
//!
//! Other crates can `use objectiveai_mcp_proxy::{ConfigBuilder, run}` and
//! spawn the proxy in-process; the binary at `main.rs` is a thin wrapper
//! that reads `Config` from the environment and calls [`run`].

mod mcp;
mod run;
mod session;
mod session_manager;
mod upstream;

use std::sync::Arc;

use objectiveai::mcp::Client;

use crate::session_manager::SessionManager;

/// Shared state every axum handler reaches via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub sessions: Arc<SessionManager>,
    pub client: Arc<Client>,
}

pub use run::*;
pub use session_manager::parse_key_env;
