//! Downloading a file or a directory out of a tool.
//!
//! A client names a tool and a path in it; the daemon sends what is
//! there — a file, or every file under a directory — one
//! [chunk](crate::daemon::download::Chunk) at a time, each a piece of
//! one file with the file's path relative to what was asked for, and
//! finishes; or answers that no tool is the one named or nothing is at
//! the path, forbidden, or that it failed, and finishes. A created
//! tool's container that is not running is started for the operation
//! and stopped when it finishes; one the daemon has running anyway — an
//! attached agent active — is used as it runs. A connected tool's
//! files are on the other daemon, and naming one is the error. Nothing
//! else about the tool changes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
