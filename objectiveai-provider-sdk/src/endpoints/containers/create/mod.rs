//! Creating a container from an image.
//!
//! Split by who SENDS: [`client`] is the caller's traffic, [`server`]
//! the provider's.
//!
//! The scope a creation opens is the container's LIFE. It carries the
//! image pull on channels the provider opens, then the container's id
//! and its filesystem on channel `0` for as long as it runs — so a
//! caller does not create and then separately ask to watch.
//!
//! The id is minted in that stream and nowhere else. Holding the scope
//! open is what keeps the container; the id is what names it to
//! anything outside.
//!
//! # Channels go both ways here
//!
//! The provider opens them to pull a caller-served image; the caller
//! opens them to reach the MCP server inside the container. Same
//! scope, opposite directions, and neither side's channel numbers
//! mean anything to the other.
//!
//! It is the clearest case for a frame layer where either end can
//! open a channel. An agentic loop only ever needed one direction; a
//! creation needs both at once.

pub mod client;
pub mod server;
