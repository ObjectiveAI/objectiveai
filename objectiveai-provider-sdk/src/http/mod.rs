//! One HTTP exchange, carried on a channel.
//!
//! Not a protocol of its own — the shape two of them travel in. A
//! provider tunnels HTTP whenever the thing it needs is on the other
//! side of a connection it cannot dial:
//!
//! - **MCP.** An agent speaks Streamable HTTP to a conduit on
//!   loopback; the exchange crosses; a client's MCP proxy terminates
//!   it.
//! - **Registry.** A provider's container runtime pulls from a
//!   registry endpoint the provider serves; the exchange crosses; a
//!   client serves the image it holds.
//!
//! Both are HTTP at BOTH ends, which is what makes the tunnel the
//! right shape. The provider is a relay in each — it originates
//! nothing and terminates nothing — so headers travel rather than
//! being invented, and a status a client chose is the status the far
//! end sees.
//!
//! # Split by direction
//!
//! [`request`] and [`response`] rather than client and server, because
//! an exchange has no sides of its own. Which end sends which is a
//! fact about the channel carrying it: an MCP request travels server
//! to client, a registry request travels the same way, and naming
//! these for sides would say nothing either way.

pub mod request;
pub mod response;
