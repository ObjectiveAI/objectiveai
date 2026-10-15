//! MCP: the agent's server at `/mcp`, every method an ask.

mod gate;
mod handler;
mod meta;
mod notifications;
mod peers;

pub use gate::*;
pub use handler::*;
pub use meta::*;
pub use notifications::*;
pub use peers::*;
