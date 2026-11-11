//! Running one command in a standalone tool's container, its output streamed.
//!
//! A client names a standalone tool and the command; the daemon runs it in the
//! container and sends its stream — stdout and stderr as they come, then
//! how it ended, as [`exec`](crate::shared::containers::exec) states it —
//! and finishes; or answers that nothing is the one named, forbidden, or
//! that it failed, and finishes. The client's cancel, or the client
//! gone, kills the command. A tool's container that is not running is started for the command, and held up while it runs. A connected tool's container is the other daemon's, and no request here finds one.
//! Split by who SENDS: the request and the cancel are in [`client`], the
//! stream in [`server`].

pub mod client;
pub mod server;
