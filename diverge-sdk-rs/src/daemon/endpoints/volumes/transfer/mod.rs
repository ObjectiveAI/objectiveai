//! Copying a file or a directory out of a volume into an agent, a tool
//! or another volume, without the client in between.
//!
//! A client names a volume, a path in it, and a
//! [`Destination`](crate::daemon::transfer::Destination); the daemon
//! takes the volume at rest, copies what is at the path — a file, or
//! every file under a directory — to the destination on its own
//! connections, and answers once that it landed, that no volume is
//! the one named or nothing is at the path, that the destination names
//! an agent, a tool or a volume that is none, that a volume at either
//! end is held, forbidden, or that it failed, and the scope finishes.
//! The bytes never reach the client. What lands where is the
//! destination's to say. The volume
//! is held for the length of the transfer and free after. An agent's or
//! a created tool's container the destination names that is not running
//! is started for the operation and stopped when it finishes; a
//! connected tool is joined for it and left.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
