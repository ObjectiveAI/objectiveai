//! Downloading a file or a directory out of a volume.
//!
//! A client names a volume and a path in it; the daemon takes the
//! volume at rest and sends what is there — a file, or every file under
//! a directory — one [chunk](crate::daemon::download::Chunk) at a time,
//! each a piece of one file with the file's path relative to what was
//! asked for, and finishes; or answers that no volume is the one named
//! or nothing is at the path, that the volume is held, forbidden, or
//! that it failed, and finishes. The volume is held for the length of
//! the download and free after; nothing is started for it.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
