//! Downloading a file or a directory out of a resource.
//!
//! A client names a resource and a path in it; the daemon sends what is
//! there — a file, or every file under a directory — one
//! [chunk](crate::daemon::download::Chunk) at a time, each a piece of
//! one file with the file's path relative to what was asked for, and
//! finishes; or answers that no resource is the one named or nothing is
//! at the path, forbidden, or that it failed, and finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
