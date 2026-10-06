//! Copying a file or a directory out of a resource into an agent or a
//! tool, without the client in between.
//!
//! A client names a resource, a path in it, and a
//! [`Destination`](crate::daemon::transfer::Destination); the daemon
//! copies what is at the path — a file, or every file under a directory
//! — to the destination on its own connections, and answers once that
//! it landed, that no resource is the one named or nothing is at the
//! path, that the destination names an agent or a tool that is none,
//! that the destination is a resource, forbidden, or that it failed,
//! and the scope finishes. The bytes never reach the client. What lands
//! where is the destination's to say. A resource is never copied into a
//! resource: a part of a directory resource wanted as a resource of its
//! own is uploaded as one, and a resource destination is answered with
//! a variant of its own. An agent's or a created tool's container the
//! destination names that is not running is started for the operation
//! and stopped when it finishes; a connected tool is joined for it and
//! left.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
