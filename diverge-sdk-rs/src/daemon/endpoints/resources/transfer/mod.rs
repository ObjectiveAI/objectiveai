//! Copying a file or a directory out of a resource into an agent, a
//! tool or a new resource, without the client in between.
//!
//! A client names a resource, a path in it, and a
//! [`Destination`](crate::daemon::transfer::Destination); the daemon
//! copies what is at the path — a file, or every file under a directory
//! — to the destination on its own connections, and answers once that
//! it landed, with the new resource's id when the destination is a
//! resource, that no resource is the one named or nothing is at the
//! path, that the destination names an agent or a tool that is none,
//! forbidden, or that it failed, and the scope finishes. The bytes
//! never reach the client. What lands where, and what a resource
//! destination makes, is the destination's to say. An agent's or a
//! created tool's container the destination names that is not running
//! is started for the operation and stopped when it finishes; a
//! connected tool is joined for it and left.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
