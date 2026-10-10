//! Copying a file or a directory out of a tool's container into an
//! agent, a tool or a volume, without the client in
//! between.
//!
//! A client names a tool, a path in it, and a
//! [`Destination`](crate::daemon::transfer::Destination); the daemon
//! copies what is at the path — a file, or every file under a directory
//! — to the destination on its own connections, and answers once that
//! it landed, that no tool is the one named or nothing is at the path,
//! that the destination names an agent, a tool or a volume that is
//! none, that a volume at either end is held, forbidden, or that it
//! failed, and the scope finishes. The bytes never reach the client.
//! What lands where is the destination's to say. A created tool's container that is not running
//! is started for the operation and stopped when it finishes; one the
//! daemon has running anyway — an attached agent active — is used as it
//! runs. A connected tool's files are on the other daemon, and naming
//! one, as the source or the destination, is the error. Nothing else
//! about the tool changes. An agent or a created tool the destination
//! names that is not running is started and stopped the same way.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
