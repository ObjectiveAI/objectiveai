//! Copying a file or a directory out of a tool's container into an
//! agent, a tool or a new resource, without the client in between.
//!
//! A client names a tool, a path in it, and a
//! [`Destination`](crate::daemon::transfer::Destination); the daemon
//! copies what is at the path — a file, or every file under a directory
//! — to the destination on its own connections, and answers once that
//! it landed, with the new resource's id when the destination is a
//! resource, that no tool is the one named or nothing is at the path,
//! that the destination names an agent or a tool that is none,
//! forbidden, or that it failed, and the scope finishes. The bytes
//! never reach the client. What lands where, and what a resource
//! destination makes, is the destination's to say. A created tool's
//! container that is not running is started for the operation and
//! stopped when it finishes; one the daemon has running anyway — an
//! attached agent active — is used as it runs. A connected tool is
//! joined for the operation through the provider protocol's
//! `containers::tools::connect` and left when it finishes. Nothing else
//! about the tool changes. An agent or a created tool the destination
//! names that is not running is started and stopped the same way; a
//! connected tool is joined and left.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
