//! Running an MCP plugin, and calling it.
//!
//! Split by who SENDS: [`client`] is the caller's traffic, [`server`]
//! the provider's.
//!
//! The scope a run opens is the plugin's LIFE. It carries the image
//! pull on channels the provider opens, and the caller's tool calls
//! ride channels of its own for as long as it holds the scope.
//!
//! Channel `0` stays quiet through all of it. A plugin that works says
//! nothing; the only thing that arrives there is a plugin that did not
//! come up, and the scope's finish is what says the run is over.
//!
//! Nothing outside that scope can reach the plugin. There is no id and
//! nothing to connect to — a plugin is created for one caller,
//! answers that caller, and is torn down after, which is what makes
//! this the only scope [`mcp_plugin`](super) has.

pub mod client;
pub mod server;
