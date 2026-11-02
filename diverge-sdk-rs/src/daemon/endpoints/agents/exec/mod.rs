//! Running one command in an agent's container, its output streamed.
//!
//! A client names an agent and the command; the daemon runs it in the
//! container and sends its stream — stdout and stderr as they come, then
//! how it ended, as [`exec`](crate::shared::containers::exec) states it —
//! and finishes; or answers that nothing is the one named, forbidden, or
//! that it failed, and finishes. The client's cancel, or the client
//! gone, kills the command. An agent's container that is not running is started for the command, and held up while it runs.
//! Split by who SENDS: the request and the cancel are in [`client`], the
//! stream in [`server`].

pub mod client;
pub mod server;
