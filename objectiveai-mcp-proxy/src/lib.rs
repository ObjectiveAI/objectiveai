//! ObjectiveAI MCP proxy library.
//!
//! Other crates can `use objectiveai_mcp_proxy::{ConfigBuilder, run}` and
//! spawn the proxy in-process; the binary at `main.rs` is a thin wrapper
//! that reads `Config` from the environment and calls [`run`].

mod logging;
mod mcp;
mod queue_delegate;
mod reverse_channel;
mod run;
mod session;
mod session_manager;
mod upstream;

use std::sync::Arc;

use objectiveai_sdk::mcp::Client;

use crate::session_manager::SessionManager;

/// Shared state every axum handler reaches via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub sessions: Arc<SessionManager>,
    pub client: Arc<Client>,
    /// Optional in-process queue-read delegate. `Some` when an
    /// embedder (the API) plugged one in at [`setup`] time;
    /// `None` for the CLI's standalone proxy — tool calls then
    /// never invoke the delegate seam.
    pub queue_delegate: Option<Arc<dyn QueueDelegate>>,
}

pub use queue_delegate::{QueueDelegate, QueueRead};
pub use run::*;
pub use session_manager::parse_key_env;
