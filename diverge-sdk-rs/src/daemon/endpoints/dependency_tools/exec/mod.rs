//! Running one command in a dependency tool's container, its output streamed.
//!
//! A client names a dependency tool and the command; the daemon runs it in the
//! container and sends its stream — stdout and stderr as they come, then
//! how it ended, as [`exec`](crate::shared::containers::exec) states it —
//! and finishes; or answers that nothing is the one named, forbidden, or
//! that it failed, and finishes. The client's cancel, or the client
//! gone, kills the command. A dependency tool that is not running is not found: nothing is started for it. One that runs is held up — with the agent it was deployed for — while the command runs.
//! Split by who SENDS: the request and the cancel are in [`client`], the
//! stream in [`server`].

pub mod client;
pub mod server;
