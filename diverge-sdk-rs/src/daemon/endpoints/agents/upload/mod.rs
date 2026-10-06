//! Uploading files into an agent's container.
//!
//! A client opens the scope saying what is coming — one file at a path,
//! or a directory of named files at a path — and the daemon asks for
//! each file's content on a channel of its own; the client answers each
//! in pieces and finishes it, and when every channel has finished the
//! daemon answers once that the files are in place, that no agent is
//! the one named, forbidden, or that it failed, and the scope finishes.
//! Each file lands whole, every parent made, replacing what was at its
//! path and touching nothing else; a content channel that ends in an
//! error abandons that file, and the answer is the error. An agent's
//! container that is not running is started for the operation and
//! stopped when it finishes; one the daemon has running anyway — a loop
//! in it — is used as it runs, and nothing else about the agent
//! changes.
//!
//! The content travels on channels the daemon opens for the reason
//! [`resources::upload`](crate::daemon::endpoints::resources::upload)
//! gives: only a responder can end a channel.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
