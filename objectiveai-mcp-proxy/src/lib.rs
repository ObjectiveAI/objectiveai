//! ObjectiveAI MCP proxy library.
//!
//! Other crates can `use objectiveai_mcp_proxy::{ConfigBuilder, run}` and
//! spawn the proxy in-process; the binary at `main.rs` is a thin wrapper
//! that reads `Config` from the environment and calls [`run`].

mod command_executor;
mod dropper;
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
    /// Optional reverse channel for `ws://` upstreams. `Some` when an
    /// embedder (the API) hands the proxy its request's reverse-attach
    /// WS at [`setup`] time; `None` for the standalone proxy, which then
    /// serves HTTP upstreams only (a `ws://` upstream errors at connect).
    pub reverse_channel: Option<ReverseChannel>,
    /// Optional dropper. `Some` when an embedder wires one in at
    /// [`setup`] time; `handle_initialize` then checks its banned set so
    /// a dropped response id is never (re)admitted. `None` → no banning.
    pub dropper: Option<Dropper>,
}

pub use dropper::Dropper;
pub use queue_delegate::{QueueDelegate, QueueRead};
pub use reverse_channel::ReverseChannel;
pub use run::*;
pub use session_manager::parse_key_env;
