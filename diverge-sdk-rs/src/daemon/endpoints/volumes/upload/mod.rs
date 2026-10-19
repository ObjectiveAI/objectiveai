//! Uploading files into a volume.
//!
//! A client opens the scope saying what is coming — one file at a path,
//! or a directory of named files at a path — and the daemon takes the
//! volume at rest and asks for each file's content on a channel of its
//! own; the client answers each in pieces and finishes it, and when
//! every channel has finished the daemon answers once that the files
//! are in place, that no volume is the one named, that the volume is
//! held, forbidden, or that it failed, and the scope finishes. Each
//! file lands whole, every parent made, replacing what was at its path
//! and touching nothing else, as the provider's
//! [`volumes::write`](crate::provider::endpoints::volumes::write) lands
//! one; a content channel that ends in an error abandons that file, and
//! the answer is the error. The volume is held for the length of the
//! upload and free after.
//!
//! The content travels on channels the daemon opens, since only a
//! responder can end a channel: the client could not say which piece
//! was its last.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
