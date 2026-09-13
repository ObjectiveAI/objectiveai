//! Running an agent container from an image.
//!
//! Split by who SENDS: [`client`] is the caller's traffic, [`server`]
//! the provider's.
//!
//! The scope a run opens is the container's LIFE. It carries the image
//! pull and the mounts' content on channels the provider opens, then
//! the container's id on channel `0`, and then the agent's
//! conversation there — every chunk the agent produces, for as long
//! as the container runs. Holding the scope open is what keeps the
//! container; the id is what names it to anything outside, and it is
//! minted in that stream and nowhere else.
//!
//! # Channels go both ways here
//!
//! The provider opens them for what it needs from the caller — an
//! image the caller serves, content it does not hold, a connector's
//! authorization — and for everything the container asks of the
//! caller: its database connections, its commands, its vault, its
//! tool calls outward. The caller opens them to reach into the
//! container: its files, its tree, a message for the agent. Same
//! scope, opposite directions, and neither side's channel numbers
//! mean anything to the other.

pub mod client;
pub mod server;
