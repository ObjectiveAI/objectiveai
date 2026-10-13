//! Downloading a file or a directory out of a dependency tool.
//!
//! A client names a dependency tool — by its template and whose it is
//! — and a path in it; the daemon sends what is there — a file, or
//! every file under a directory — one
//! [chunk](crate::daemon::download::Chunk) at a time, each a piece of
//! one file with the file's path relative to what was asked for, and
//! finishes; or answers that no dependency tool is the one named or
//! none runs, or that nothing is at the path, forbidden, or that it
//! failed, and finishes. The tool's container is read as it runs: a
//! dependency tool that does not run is not started for the
//! operation, and is `NotFound`; nothing is stopped or released when
//! the operation ends. Nothing else about the tool changes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
