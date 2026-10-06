//! Copying a file or a directory out of an agent's container into an
//! agent, a tool, a volume or a new resource, without the client in
//! between.
//!
//! A client names an agent of the caller's, a path in it, and a
//! [`Destination`](crate::daemon::transfer::Destination); the daemon
//! copies what is at the path — a file, or every file under a directory
//! — to the destination on its own connections, and answers once that
//! it landed, with the new resource's id when the destination is a
//! resource, that no agent is the one named or nothing is at the path,
//! that the destination names an agent, a tool or a volume that is
//! none, that a volume at either end is held, forbidden, or that it
//! failed, and the scope finishes. The bytes never reach the client.
//! What lands where, and what a resource destination makes, is the
//! destination's to say. An agent's container that is not running is
//! started for the operation and stopped when it finishes; one the
//! daemon has running anyway — a loop in it — is used as it runs, and
//! nothing else about the agent changes. An agent or a created tool the
//! destination names that is not running is started and stopped the
//! same way; a connected tool is joined and left.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
