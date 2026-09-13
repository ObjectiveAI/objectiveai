//! MCP: the agent's server at `/mcp`, every method an ask.

mod gate;
mod handler;
mod notifications;
mod peers;

pub use gate::*;
pub use handler::*;
pub use notifications::*;
pub use peers::*;
