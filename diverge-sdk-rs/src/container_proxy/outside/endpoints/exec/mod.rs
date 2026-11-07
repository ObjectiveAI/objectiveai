//! Running one command in the container, its output streamed.
//!
//! The server names the command and gets its output — stdout and
//! stderr as they come, then how it ended — until the command ends or
//! the server cancels it, which kills it. Split by who SENDS: the
//! request and the cancel go in [`client`], the stream comes back in
//! [`server`]. The command, its stream and its rules are
//! [`exec`](crate::shared::containers::exec)'s, not a second
//! description of the same thing.

pub mod client;
pub mod server;
