//! The answers a provider sends on the channels a caller opened.
//!
//! [`mcp`] is what the plugin's MCP server said. [`postgres`] is what
//! the plugin wrote toward the caller's database.
//!
//! Two, and they are not two of a kind. An MCP channel is an exchange
//! the caller initiated and this answers. A Postgres channel is one
//! the caller opened only so that the provider would have something to
//! answer ON — the connection itself was the provider's idea, and the
//! writes are a stream that has to be endable by the side producing
//! them.
//!
//! One module per kind of channel, each naming its own type `Frame`.
//! Nothing is re-exported upward: the module is the only thing telling
//! them apart, so it has to stay in the path.

pub mod mcp;
pub mod postgres;
