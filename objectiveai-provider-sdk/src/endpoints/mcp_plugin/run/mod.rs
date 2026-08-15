//! Running an MCP plugin, and calling it.
//!
//! Split by who SENDS: [`client`] is the caller's traffic, [`server`]
//! the provider's.
//!
//! The scope a run opens is the plugin's LIFE. It carries the image
//! pull on channels the provider opens, then says the plugin is up on
//! channel `0`, and the caller's tool calls ride channels of its own
//! for as long as it holds the scope.
//!
//! Nothing outside that scope can reach the plugin. There is no id and
//! nothing to connect to — a plugin is created for one caller,
//! answers that caller, and is torn down after, which is what makes
//! this the only scope [`mcp_plugin`](super) has.

pub mod client;
pub mod server;
