//! The channels a client opens on a provider during a creation.
//!
//! One, and it is the mirror of the agentic loop's. There, a provider
//! runs the agent and the MCP servers live with the client, so the
//! provider opens channels outward. Here the container runs on the
//! PROVIDER, so the MCP server it holds is the thing a caller cannot
//! dial — and the caller opens the channels instead.
//!
//! Same protocol, same frames, opposite direction. Which is the point
//! of a frame layer that lets either side open one.

mod frame;

pub use frame::*;
